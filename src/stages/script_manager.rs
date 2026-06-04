use std::env;
use std::io::{stdin, stdout, BufRead, BufReader, Stdout};
use std::os::fd::FromRawFd;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use arrow::ipc::reader::StreamReader;
use arrow::ipc::writer::StreamWriter;
use crate::stages::manager::Manager;
use crate::stages::WorkerMode;

const PROFILE_BATCHES: usize = 5;

fn trimmed_mean_ms(times: &[Duration]) -> f64 {
    let mut sorted = times.to_vec();
    sorted.sort();
    let trimmed = &sorted[1..sorted.len() - 1];
    let sum: Duration = trimmed.iter().sum();
    sum.as_secs_f64() * 1000.0 / trimmed.len() as f64
}

fn spawn_scale_reader() -> mpsc::Receiver<usize> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let file = unsafe { std::fs::File::from_raw_fd(3) };
        let mut line = String::new();
        BufReader::new(file).read_line(&mut line).ok();
        if let Ok(n) = line.trim().parse::<usize>() {
            let _ = tx.send(n);
        }
    });
    rx
}

pub fn run_manager(path: &String, flags: Vec<(String, String)>, index: usize, worker_mode: WorkerMode) -> anyhow::Result<()> {
    let auto_alloc = env::var("RIL_AUTO_ALLOC").is_ok();

    let num_workers: usize = if auto_alloc {
        1
    } else {
        match worker_mode {
            WorkerMode::Default => 1,
            WorkerMode::Fixed(n) => n,
            WorkerMode::Dynamic => num_cpus::get(),
        }
    };

    let (exe, is_python) = if path.ends_with(".py") {
        (env::current_exe()?, true)
    } else {
        (PathBuf::from(path), false)
    };

    let mut reader = StreamReader::try_new(stdin(), None)?;
    let mut writer: Option<StreamWriter<Stdout>> = None;
    let mut manager = Manager::new(exe, num_workers, flags, index, is_python)?;

    let mut batch_times: Vec<Duration> = Vec::with_capacity(PROFILE_BATCHES);
    let mut timed = !auto_alloc;
    let mut scale_rx: Option<mpsc::Receiver<usize>> = None;

    for batch in &mut reader {
        let t = Instant::now();
        let result = manager.run_thread_work(batch?)?;
        let elapsed = t.elapsed();

        if !timed {
            batch_times.push(elapsed);
            if batch_times.len() == PROFILE_BATCHES {
                let mean_ms = trimmed_mean_ms(&batch_times);
                eprintln!("RIL_TIMING:{mean_ms}");
                scale_rx = Some(spawn_scale_reader());
                timed = true;
            }
        }

        if let Some(ref rx) = scale_rx {
            if let Ok(target) = rx.try_recv() {
                manager.scale_to(target)?;
                scale_rx = None;
            }
        }

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
