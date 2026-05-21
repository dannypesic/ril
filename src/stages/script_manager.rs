use std::env;
use std::io::{stdin, stdout, Stdout};
use std::path::PathBuf;
use arrow::ipc::reader::StreamReader;
use arrow::ipc::writer::StreamWriter;
use crate::stages::manager::Manager;

pub fn run_manager(path: &String, flags: Vec<(String, String)>, index: usize) -> anyhow::Result<()> {
    
    let num_workers: usize = 8; //fixed num for now
    let (exe, worker_args) = if path.ends_with(".py") {
        (env::current_exe()?, vec![
            ("--stage-index".to_string(), index.to_string()),
            ("--worker-is-python".to_string(), "true".to_string()),
        ])
    } else {
        (PathBuf::from(path), flags)
    };

    let mut reader = StreamReader::try_new(stdin(), None)?;
    let mut writer: Option<StreamWriter<Stdout>> = None;
    let mut manager: Manager = Manager::new(exe, num_workers, worker_args)?;

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
