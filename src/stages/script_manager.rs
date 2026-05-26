use std::env;
use std::io::{stdin, stdout, Stdout};
use std::path::PathBuf;
use arrow::ipc::reader::StreamReader;
use arrow::ipc::writer::StreamWriter;
use crate::stages::manager::Manager;
use crate::stages::WorkerMode;

pub fn run_manager(path: &String, flags: Vec<(String, String)>, index: usize, worker_mode: WorkerMode) -> anyhow::Result<()> {

    let num_workers: usize = match worker_mode {
        WorkerMode::Default => 1,
        WorkerMode::Fixed(n) => n,
        WorkerMode::Dynamic => num_cpus::get(),
    };
    let (exe, is_python) = if path.ends_with(".py") {
        (env::current_exe()?, true)
    } else {
        (PathBuf::from(path), false)
    };

    let mut reader = StreamReader::try_new(stdin(), None)?;
    let mut writer: Option<StreamWriter<Stdout>> = None;
    let mut manager: Manager = Manager::new(exe, num_workers, flags, index, is_python)?;

    for batch in &mut reader {
        let result = manager.run_thread_work(batch?)?;
        let w = match writer {
            Some(ref mut w) => w,
            None => {
                writer = Some(StreamWriter::try_new(stdout(), result.schema_ref())?);
                writer.as_mut().unwrap()
            }
        };
        w.write(&result)?;
    }
    if let Some(mut w) = writer {
        w.finish()?;
    }

    manager.kill()?;
    Ok(())
}
