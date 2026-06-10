use std::collections::HashSet;
use std::io::{pipe, BufRead, BufReader, Write};
use std::os::fd::OwnedFd;
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use command_fds::{CommandFdExt, FdMapping};
use crate::error::ERROR_PREFIX;
use crate::pipeline::progress;
use crate::stages::{BuiltinStage, ScriptStage, Stage, WorkerMode};

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

    let stage_names: Vec<String> = stages.iter().map(|s| match s {
        Stage::Builtin(BuiltinStage::Load { path, .. }) => format!("load {path}"),
        Stage::Builtin(BuiltinStage::Save { path }) => format!("save {path}"),
        Stage::Builtin(BuiltinStage::Tee { path }) => format!("tee {path}"),
        Stage::Script(ScriptStage { path, .. }) => path.clone(),
    }).collect();

    let load_idx: Option<usize> = stages.iter().position(|s| {
        matches!(s, Stage::Builtin(BuiltinStage::Load { .. }))
    });
    let script_indices: Vec<usize> = stages.iter().enumerate()
        .filter(|(_, s)| matches!(s, Stage::Script(_)))
        .map(|(i, _)| i)
        .collect();
    // Last non-tee stage is where we read RIL_BATCH for overall progress.
    // Prefer save → last script → load, naturally falls out of this filter.
    let progress_stage_idx: Option<usize> = stages.iter().enumerate().rev()
        .find(|(_, s)| !matches!(s, Stage::Builtin(BuiltinStage::Tee { .. })))
        .map(|(i, _)| i);

    let capture_stderr: HashSet<usize> = {
        let mut s = HashSet::new();
        if let Some(i) = load_idx { s.insert(i); }
        for &i in &script_indices { s.insert(i); }
        if let Some(i) = progress_stage_idx { s.insert(i); }
        s
    };

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

        if capture_stderr.contains(&index) {
            cmd.stderr(Stdio::piped());
        }

        if auto_mode && is_script {
            let (ctrl_r, ctrl_w) = pipe()?;
            control_writers.push(Some(ctrl_w));
            cmd.env("RIL_AUTO_ALLOC", "1")
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

    let (progress_tx, progress_rx) = mpsc::channel::<progress::Msg>();
    let script_count = script_indices.len();

    let (timing_tx_opt, timing_rx_opt) = if auto_mode {
        let (tx, rx) = mpsc::channel::<(usize, f64)>();
        (Some(tx), Some(rx))
    } else {
        (None, None)
    };

    for (index, child) in children.iter_mut().enumerate() {
        if !capture_stderr.contains(&index) {
            continue;
        }
        let Some(stderr) = child.stderr.take() else { continue };

        let is_load = Some(index) == load_idx;
        let is_progress = Some(index) == progress_stage_idx;
        let is_script_stage = script_indices.contains(&index);

        let progress_tx_clone = if is_load || is_progress { Some(progress_tx.clone()) } else { None };
        let timing_tx_clone = timing_tx_opt.as_ref()
            .filter(|_| is_script_stage)
            .cloned();
        let stage_name = stage_names[index].clone();

        thread::spawn(move || {
            let mut timing_sent = false;
            for line in BufReader::new(stderr).lines().flatten() {
                if let Some(rest) = line.strip_prefix("RIL_TOTAL_BATCHES:") {
                    if is_load {
                        if let (Some(tx), Ok(n)) = (&progress_tx_clone, rest.parse::<usize>()) {
                            let _ = tx.send(progress::Msg::Total(n));
                        }
                    }
                } else if let Some(rest) = line.strip_prefix("RIL_BATCH:") {
                    if is_progress {
                        if let (Some(tx), Ok(n)) = (&progress_tx_clone, rest.parse::<usize>()) {
                            let _ = tx.send(progress::Msg::Batch(n));
                        }
                    }
                } else if let Some(rest) = line.strip_prefix("RIL_TIMING:") {
                    if !timing_sent {
                        if let (Some(tx), Ok(ms)) = (&timing_tx_clone, rest.parse::<f64>()) {
                            let _ = tx.send((index, ms));
                            timing_sent = true;
                        }
                    }
                } else if let Some(rest) = line.strip_prefix(ERROR_PREFIX) {
                    eprintln!("error in stage[{index}] `{stage_name}`: {rest}");
                } else {
                    eprintln!("{line}");
                }
            }
        });
    }

    drop(progress_tx);
    drop(timing_tx_opt);

    let progress_handle = thread::spawn(move || {
        progress::run(progress_rx);
    });

    if auto_mode {
        let timing_rx = timing_rx_opt.unwrap();
        thread::spawn(move || {
            let mut timings: Vec<(usize, f64)> = Vec::new();
            for _ in 0..script_count {
                match timing_rx.recv() {
                    Ok(t) => timings.push(t),
                    Err(_) => break,
                }
            }
            if timings.is_empty() { return; }
            let allocs = allocate_workers(&timings, num_cpus::get());
            for (stage_idx, count) in allocs {
                if let Some(Some(mut w)) = control_writers.get_mut(stage_idx).map(|e| e.take()) {
                    let _ = write!(w, "{count}\n");
                }
            }
        });
    }

    for (index, child) in children.iter_mut().enumerate() {
        let status = child.wait()?;
        if !status.success() {
            let name = &stage_names[index];
            let code = status.code().map(|c| c.to_string()).unwrap_or_else(|| "signal".into());
            anyhow::bail!("stage[{index}] `{name}` exited with code {code}");
        }
    }

    progress_handle.join().ok();

    Ok(())
}
