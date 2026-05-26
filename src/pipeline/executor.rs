use std::process::{Command, Stdio, Child, ChildStdout};
use crate::stages::Stage;

pub fn run(stages: Vec<Stage>) -> anyhow::Result<()> {
    if stages.is_empty() {
        anyhow::bail!("empty pipeline");
    }

    let exe = std::env::current_exe()?;
    let mut children: Vec<Child> = Vec::new();

    let mut prev_stdout: Option<ChildStdout> = None;

    for index in 0..stages.len() {
        let stdin = match prev_stdout.take() {
            Some(out) => Stdio::from(out),
            None => Stdio::null(),
        };
        let venv = std::env::current_dir()?.join(".venv");
        let mut child = Command::new(&exe)
            .env("RIL_STAGE_INDEX", index.to_string())
            .env("VIRTUAL_ENV", venv)
            .stdin(stdin)
            .stdout(Stdio::piped())
            .spawn()?;
        prev_stdout = child.stdout.take();
        children.push(child);
    }

    for child in &mut children {
        child.wait()?;
    }

    Ok(())
}