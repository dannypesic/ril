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
                    "-m",
                    "venv",
                    ".venv"
                ])
                .output().expect("couldn't create venv :(");
        } else {
            Command::new("python")
                .args(vec![
                    "-m",
                    "venv",
                    ".venv"
                ])
                .output().expect("couldn't create venv :(");
        }
    }

    Command::new(".venv/bin/pip3")
        .args(vec![
            "install",
            "pyarrow",
            "arro3-core"
        ])
        .output().expect("couldn't install dependencies :(");

    if !Path::new("ril.py").is_file() {
        std::fs::write("ril.py", "def rilfn(fn):\n    import sys\n    frame = sys._getframe(1)\n    frame.f_globals['__ril_main__'] = fn\n    return fn\n")
            .expect("couldn't create ril.py :(");
    }

    Ok(())
}