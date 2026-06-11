use std::path::Path;
use std::process::Command;

enum Manager {
    Pip,
    Uv
}

fn find_python() -> anyhow::Result<String> {
    let versioned = ["python3.14", "python3.13", "python3.12", "python3.11"];

    for candidate in &versioned {
        if interpreter_version_ok(candidate) {
            return Ok(candidate.to_string());
        }
    }

    if interpreter_version_ok("python3") {
        return Ok("python3".to_string());
    }

    anyhow::bail!(
        "ril requires Python 3.11 or newer, but none was found on PATH.\n\
         Tried: {versioned}, python3.\n\
         Install Python 3.11–3.14 and make sure it is on your PATH.",
        versioned = versioned.join(", ")
    )
}

fn interpreter_version_ok(interpreter: &str) -> bool {
    let output = match Command::new(interpreter).arg("--version").output() {
        Ok(o) => o,
        Err(_) => return false,
    };

    if !output.status.success() {
        return false;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    parse_version_ok(&text)
}

fn parse_version_ok(text: &str) -> bool {
    let version_str = text
        .split_whitespace()
        .nth(1)
        .unwrap_or("");
    let parts: Vec<&str> = version_str.split('.').collect();
    if parts.len() < 2 {
        return false;
    }
    let major: u32 = parts[0].parse().unwrap_or(0);
    let minor: u32 = parts[1].parse().unwrap_or(0);
    major == 3 && minor >= 11
}

pub fn setup_env() -> anyhow::Result<()> {

    let manager: Manager = if Path::new("uv.lock").is_file() {
        Manager::Uv
    } else {
        Manager::Pip
    };

    if !Path::new(".venv").is_dir() {

        match manager {
            Manager::Uv => {
                Command::new("uv")
                    .arg("venv")
                    .output()
                    .expect("couldn't create uv venv :(");
            },
            Manager::Pip => {
                let python = find_python()?;

                let status = Command::new(&python)
                    .args(["-m", "venv", ".venv"])
                    .status()
                    .map_err(|e| anyhow::anyhow!("failed to run `{python} -m venv .venv`: {e}"))?;

                anyhow::ensure!(
                    status.success(),
                    "couldn't create .venv — `{python} -m venv .venv` exited with {status}"
                );
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
                .args(["pip", "install", "pyarrow", "arro3-core"])
                .output().expect("uv couldn't install dependencies :(");
        },
        Manager::Pip => {
            Command::new(".venv/bin/pip3")
                .args(["install", "pyarrow", "arro3-core"])
                .output().expect("pip couldn't install dependencies :(");
        }
    }

    if !Path::new("ril.py").is_file() {
        std::fs::write("ril.py", "def rilfn(fn):\n    import sys\n    frame = sys._getframe(1)\n    frame.f_globals['__ril_main__'] = fn\n    return fn\n")
            .expect("couldn't create ril.py :(");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_version_ok;

    #[test]
    fn accepts_311_and_above() {
        assert!(parse_version_ok("Python 3.11.0"));
        assert!(parse_version_ok("Python 3.12.3"));
        assert!(parse_version_ok("Python 3.13.0rc1"));
        assert!(parse_version_ok("Python 3.14.0a5"));
    }

    #[test]
    fn rejects_below_311() {
        assert!(!parse_version_ok("Python 3.10.12"));
        assert!(!parse_version_ok("Python 3.9.7"));
        assert!(!parse_version_ok("Python 2.7.18"));
    }

    #[test]
    fn rejects_garbage() {
        assert!(!parse_version_ok(""));
        assert!(!parse_version_ok("not a version"));
    }
}
