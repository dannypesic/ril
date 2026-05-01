use crate::stages::{Stage, builtins::run_stage};
use crate::buffer::{Sender, Receiver, channel};

pub fn run(stages: Vec<Stage>) -> anyhow::Result<()> {
    if stages.is_empty() {
        anyhow::bail!("empty pipeline");
    }

    let mut channels: Vec<(Sender, Receiver)> =
        (0..(stages.len() - 1)).map(|_| channel()).collect();

    let mut handles = Vec::new();

    for (i, stage) in stages.into_iter().enumerate() {
        let rx = 
            if i == 0 { 
                None 
            } else {
                Some(channels[i - 1].1.clone())
            };
        let tx = 
            if i == channels.len() {
                None
            } else {
                Some(channels[i].0.clone())
            };

        let handle = std::thread::spawn(move || {
            run_stage(stage, rx, tx) // this should be a process, not a thread (when flagged) for proper parallel computed and no GIL
        });
        handles.push(handle);
    }

    for handle in handles { //again, migrate to process in future.
        handle.join().unwrap()?;
    }

    Ok(())
}