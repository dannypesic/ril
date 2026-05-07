use std::process::{Command, Stdio, Child, ChildStdout};
use crate::stages::Stage;

fn stage_to_args(index: &usize) -> Vec<String> {
    vec!["--stage-index".to_string(), index.to_string()]
}

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
        let mut child = Command::new(&exe)
            .args(stage_to_args(&index))
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