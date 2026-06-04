use std::io::{pipe, BufRead, BufReader, Write};
use std::os::fd::OwnedFd;
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use command_fds::{CommandFdExt, FdMapping};
use crate::stages::{ScriptStage, Stage, WorkerMode};

fn is_auto_mode(stages: &[Stage]) -> bool {
    let has_scripts = stages.iter().any(|s| matches!(s, Stage::Script(_)));
    let has_explicit = stages.iter().any(|s| matches!(
        s,
        Stage::Script(ScriptStage { workers: WorkerMode::Fixed(_) | WorkerMode::Dynamic, .. })
    ));
    has_scripts && !has_explicit
}

fn allocate_workers(timings: &[(usize, f64)], total: usize) -> Vec<(usize, usize)> {
    let n = timings.len();

    if total <= n {
        return timings.iter().map(|&(idx, _)| (idx, 1)).collect();
    }

    // Give every stage 1 guaranteed worker, then distribute the remainder proportionally.
    // This keeps the sum exactly equal to `total` regardless of how skewed the timings are.
    let extra = total - n;
    let total_time: f64 = timings.iter().map(|(_, t)| t).sum();

    let mut allocs: Vec<(usize, usize, f64)> = timings.iter()
        .map(|&(idx, t)| {
            let exact = extra as f64 * t / total_time;
            (idx, 1 + exact as usize, exact.fract())
        })
        .collect();

    let assigned: usize = allocs.iter().map(|(_, w, _)| w).sum();
    let mut remainder = total.saturating_sub(assigned);

    allocs.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());
    for entry in allocs.iter_mut() {
        if remainder == 0 { break; }
        entry.1 += 1;
        remainder -= 1;
    }

    allocs.into_iter().map(|(idx, w, _)| (idx, w)).collect()
}

pub fn run(stages: Vec<Stage>) -> anyhow::Result<()> {
    if stages.is_empty() {
        anyhow::bail!("empty pipeline");
    }

    let auto_mode = is_auto_mode(&stages);
    let exe = std::env::current_exe()?;
    let venv = std::env::current_dir()?.join(".venv");

    let mut children: Vec<Child> = Vec::new();
    let mut prev_stdout: Option<ChildStdout> = None;
    let mut control_writers: Vec<Option<std::io::PipeWriter>> = Vec::new();

    for (index, stage) in stages.iter().enumerate() {
        let is_script = matches!(stage, Stage::Script(_));

        let stdin = match prev_stdout.take() {
            Some(out) => Stdio::from(out),
            None => Stdio::null(),
        };

        let mut cmd = Command::new(&exe);
        cmd.env("RIL_STAGE_INDEX", index.to_string())
            .env("VIRTUAL_ENV", &venv)
            .stdin(stdin)
            .stdout(Stdio::piped());

        if auto_mode && is_script {
            let (ctrl_r, ctrl_w) = pipe()?;
            control_writers.push(Some(ctrl_w));

            cmd.env("RIL_AUTO_ALLOC", "1")
                .stderr(Stdio::piped())
                .fd_mappings(vec![FdMapping {
                    parent_fd: OwnedFd::from(ctrl_r),
                    child_fd: 3,
                }])?;
        } else {
            control_writers.push(None);
        }

        let mut child = cmd.spawn()?;
        prev_stdout = child.stdout.take();
        children.push(child);
    }

    if auto_mode {
        let script_indices: Vec<usize> = stages.iter().enumerate()
            .filter(|(_, s)| matches!(s, Stage::Script(_)))
            .map(|(i, _)| i)
            .collect();

        let stage_stderrs: Vec<(usize, std::process::ChildStderr)> = children.iter_mut()
            .enumerate()
            .filter(|(i, _)| script_indices.contains(i))
            .map(|(i, c)| (i, c.stderr.take().unwrap()))
            .collect();

        let script_count = stage_stderrs.len();
        let (timing_tx, timing_rx) = mpsc::channel::<(usize, f64)>();

        for (stage_idx, stderr) in stage_stderrs {
            let timing_tx = timing_tx.clone();
            thread::spawn(move || {
                let mut sent = false;
                for line in BufReader::new(stderr).lines().flatten() {
                    if !sent {
                        if let Some(rest) = line.strip_prefix("RIL_TIMING:") {
                            if let Ok(ms) = rest.parse::<f64>() {
                                let _ = timing_tx.send((stage_idx, ms));
                                sent = true;
                                continue;
                            }
                        }
                    }
                    eprintln!("{line}");
                }
            });
        }
        drop(timing_tx);

        thread::spawn(move || {
            let mut timings: Vec<(usize, f64)> = Vec::new();
            for _ in 0..script_count {
                match timing_rx.recv() {
                    Ok(t) => timings.push(t),
                    Err(_) => break,
                }
            }

            if timings.is_empty() {
                return;
            }

            let allocs = allocate_workers(&timings, num_cpus::get());
            for (stage_idx, count) in allocs {
                if let Some(Some(mut w)) = control_writers.get_mut(stage_idx).map(|e| e.take()) {
                    let _ = write!(w, "{count}\n");
                }
            }
        });
    }

    for child in &mut children {
        child.wait()?;
    }

    Ok(())
}