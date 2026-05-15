use std::path::Path;
use std::process::Command;

enum Manager {
    Pip,
    Uv
}

pub fn setup_env() -> anyhow::Result<()> {

    let manager : Manager = if Path::new("uv.lock").is_file() {
        Manager::Uv
    } else {
        Manager::Pip
    };

    if ! Path::new(".venv").is_dir() {

        match manager {
            Manager::Uv => {
                Command::new("uv")
                    .arg("venv")
                    .output()
                    .expect("couldn't create uv venv :(");
            },
            Manager::Pip => {
                let which_py = if Command::new("python3")
                    .arg("--version")
                    .output()
                    .is_ok()
                { "python3" } else {"python"};

                Command::new(which_py)
                    .args(vec![
                        "-m",
                        "venv",
                        ".venv"
                    ])
                    .output()
                    .expect(format!("couldn't create venv :( {which_py} not found").as_str());
            }
        }
    }

    let venv_path = std::fs::canonicalize(".venv")
        .unwrap_or_else(|_| std::path::PathBuf::from(".venv"));
    unsafe {
        std::env::set_var("VIRTUAL_ENV", &venv_path);
    }

    match manager {
        Manager::Uv => {
            Command::new("uv")
                .args(vec![
                    "pip",
                    "install",
                    "pyarrow",
                    "arro3-core"
                ])
                .output().expect("uv couldn't install dependencies :(");
        },
        Manager::Pip => {
            Command::new(".venv/bin/pip3")
                .args(vec![
                    "install",
                    "pyarrow",
                    "arro3-core"
                ])
                .output().expect("pip couldn't install dependencies :(");
        }
    }

    if !Path::new("ril.py").is_file() {
        std::fs::write("ril.py", "def rilfn(fn):\n    import sys\n    frame = sys._getframe(1)\n    frame.f_globals['__ril_main__'] = fn\n    return fn\n")
            .expect("couldn't create ril.py :(");
    }

    Ok(())
}