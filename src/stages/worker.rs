use std::io::{pipe, PipeReader, PipeWriter};
use std::os::fd::OwnedFd;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::mpsc;
use std::thread;
use arrow::array::RecordBatch;
use arrow::ipc::reader::StreamReader;
use arrow::ipc::writer::StreamWriter;
use command_fds::{CommandFdExt, FdMapping};

fn flatten_flags(flags: &[(String, String)]) -> Vec<&str> {
    flags.iter()
        .flat_map(|(k, v)| {
            if v.is_empty() {
                vec![k.as_str()]
            } else {
                vec![k.as_str(), v.as_str()]
            }
        })
        .collect()
}

pub struct Worker {
    pub process: Child,
    pub(crate) thread: thread::JoinHandle<()>,
    pub tx: mpsc::Sender<RecordBatch>,
}
impl Worker {
    pub(crate) fn new(exe: &PathBuf, index: usize, flags: &Vec<(String, String)>, result_tx: mpsc::Sender<(usize, RecordBatch)>) -> anyhow::Result<Self> {
        let (mgr_rx, worker_tx) = pipe()?;
        let (worker_rx, mgr_tx) = pipe()?;
        let (tx, rx) = mpsc::channel();

        let process = Command::new(&exe)
            .fd_mappings(vec![
                FdMapping { parent_fd: OwnedFd::from(worker_rx.try_clone()?), child_fd: 0 },
                FdMapping { parent_fd: OwnedFd::from(worker_tx.try_clone()?), child_fd: 1 },
            ])?
            .args(flatten_flags(&flags))
            .spawn()?;
        drop(worker_rx);
        drop(worker_tx);
        Ok(Self {
            process,
            thread: thread::spawn(
                move || {
                    Self::thread_exec(rx, mgr_rx, mgr_tx, result_tx, index).unwrap()
                }
            ),
            tx,
        })
    }

    fn thread_exec(rx: mpsc::Receiver<RecordBatch>, mgr_rx: PipeReader, mgr_tx: PipeWriter, result_tx: mpsc::Sender<(usize, RecordBatch)>, index: usize) -> anyhow::Result<()> {

        let mut writer: Option<StreamWriter<PipeWriter>> = None;
        let mut mgr_tx = Some(mgr_tx);
        let mut reader = StreamReader::try_new(mgr_rx, None)?;


        loop {
            match rx.recv() {
                Ok(data) => {
                    let w = match writer {
                        Some(ref mut w) => w,
                        None => {
                            writer = Some(StreamWriter::try_new(mgr_tx.take().expect("mgr_tx already consumed"), data.schema_ref())?);
                            writer.as_mut().unwrap()
                        }
                    };
                    w.write(&data)?;
                    let batch = reader.next()
                        .ok_or_else(|| anyhow::anyhow!("worker closed pipe early"))??;
                    result_tx.send((index, batch))?; }
                Err(_) => {
                    break;
                }
            }
        }
        if let Some(mut w) = writer {
            w.finish()?;
        }
        Ok(())
    }
}
