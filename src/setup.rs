use std::path::Path;
use std::process::Command;

pub fn setup_env() -> anyhow::Result<()> {

    if ! Path::new(".venv").is_dir() {
        if Command::new("python3")
            .arg("--version")
            .output()
            .is_ok()
        {
            Command::new("python3")
                .args(vec![
                    "-m".to_string(),
                    "venv".to_string(),
                    ".venv".to_string()
                ])
                .output().expect("couldn't create venv :(");
        } else {
            Command::new("python")
                .args(vec![
                    "-m".to_string(),
                    "venv".to_string(),
                    ".venv".to_string()
                ])
                .output().expect("couldn't create venv :(");
        }
    }

    Command::new(".venv/bin/pip3")
        .args(vec![
            "install".to_string(),
            "pyarrow".to_string(),
            "arro3-core".to_string()
        ])
        .output().expect("couldn't install dependencies :(");

    if ! Path::new("ril.py").is_file() {
        Command::new("echo")
            .args(vec![
                "-m".to_string(),
                ">".to_string(),
                "ril.py".to_string()
            ])
            .output().expect("couldn't create ril.py :(");
    }

    Ok(())
}