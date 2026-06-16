use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;
use tempfile::TempDir;

fn ril_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_ril"))
}

fn shared_venv() -> &'static PathBuf {
    static VENV: OnceLock<PathBuf> = OnceLock::new();
    VENV.get_or_init(|| {
        let dir = tempfile::Builder::new()
            .prefix("ril-test-venv-")
            .tempdir()
            .expect("create shared venv tmpdir")
            .keep();

        let which_py = if Command::new("python3").arg("--version").output().is_ok() {
            "python3"
        } else {
            "python"
        };

        let status = Command::new(which_py)
            .args(["-m", "venv", dir.to_str().unwrap()])
            .status()
            .expect("create venv");
        assert!(status.success(), "venv creation failed");

        let status = Command::new(dir.join("bin/pip3"))
            .args([
                "install",
                "-q",
                "--disable-pip-version-check",
                "pyarrow",
                "arro3-core",
            ])
            .status()
            .expect("pip install");
        assert!(status.success(), "pip install pyarrow arro3-core failed");

        dir
    })
}

fn make_dir() -> TempDir {
    let dir = TempDir::new().unwrap();
    std::os::unix::fs::symlink(shared_venv(), dir.path().join(".venv"))
        .expect("symlink .venv");
    fs::write(
        dir.path().join("ril.py"),
        "def rilfn(fn):\n\
         \timport sys\n\
         \tframe = sys._getframe(1)\n\
         \tframe.f_globals['__ril_main__'] = fn\n\
         \treturn fn\n",
    )
    .unwrap();
    dir
}

fn run_ril(dir: &TempDir) -> std::process::Output {
    Command::new(ril_bin())
        .current_dir(dir.path())
        .output()
        .expect("spawn ril")
}

#[test]
fn test_load_save_roundtrip() {
    let dir = make_dir();
    fs::write(dir.path().join("input.csv"), "name,value\nalice,1\nbob,2\n").unwrap();
    fs::write(dir.path().join("rilfile"), "load input.csv | save output.csv").unwrap();

    let out = run_ril(&dir);
    assert!(
        out.status.success(),
        "ril failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );

    let csv = fs::read_to_string(dir.path().join("output.csv"))
        .expect("output.csv not created");
    let rows: Vec<&str> = csv.lines().filter(|l| !l.is_empty()).collect();

    assert_eq!(rows.len(), 3, "expected header + 2 data rows\ngot: {csv:?}");
    assert!(
        csv.contains("name") && csv.contains("value"),
        "column headers missing: {csv:?}"
    );
    assert!(
        csv.contains("alice") && csv.contains("bob"),
        "data rows missing: {csv:?}"
    );
}

#[test]
fn test_python_stage_adds_column() {
    let dir = make_dir();
    fs::write(dir.path().join("input.csv"), "x,y\n3,4\n5,8\n").unwrap();

    fs::write(
        dir.path().join("transform.py"),
        r#"from ril import rilfn
import pyarrow as pa

@rilfn
def process(batch):
    batch = pa.record_batch(batch)
    data = batch.to_pydict()
    data['doubled_x'] = [v * 2 for v in data['x']]
    return pa.RecordBatch.from_pydict(data)
"#,
    )
    .unwrap();

    fs::write(
        dir.path().join("rilfile"),
        "load input.csv | transform.py x1 | save output.csv",
    )
    .unwrap();

    let out = run_ril(&dir);
    assert!(
        out.status.success(),
        "ril failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );

    let csv = fs::read_to_string(dir.path().join("output.csv"))
        .expect("output.csv not created");

    assert!(
        csv.contains("doubled_x"),
        "'doubled_x' column missing from output: {csv}"
    );
    assert!(csv.contains('6'), "expected doubled value 6 in output: {csv}");
    assert!(csv.contains("10"), "expected doubled value 10 in output: {csv}");
}

#[test]
fn test_tee_writes_intermediate_file() {
    let dir = make_dir();
    fs::write(dir.path().join("input.csv"), "id,score\n1,100\n2,200\n").unwrap();
    fs::write(
        dir.path().join("rilfile"),
        "load input.csv | tee mid.csv | save output.csv",
    )
    .unwrap();

    let out = run_ril(&dir);
    assert!(
        out.status.success(),
        "ril failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );

    for name in ["mid.csv", "output.csv"] {
        let content = fs::read_to_string(dir.path().join(name))
            .unwrap_or_else(|_| panic!("{name} was not created by ril"));
        assert!(
            content.contains("id") && content.contains("score"),
            "{name}: header columns missing\ncontent: {content:?}"
        );
        assert!(
            content.contains("100") && content.contains("200"),
            "{name}: data values missing\ncontent: {content:?}"
        );
    }
}

#[test]
fn test_binary_stage_passthrough() {
    let dir = make_dir();
    fs::write(dir.path().join("input.csv"), "x,y\n10,20\n30,40\n").unwrap();

    let src = PathBuf::from(env!("CARGO_BIN_EXE_passthrough"));
    let dst = dir.path().join("passthrough");
    fs::copy(&src, &dst).unwrap();
    fs::set_permissions(&dst, fs::Permissions::from_mode(0o755)).unwrap();

    fs::write(
        dir.path().join("rilfile"),
        "load input.csv | ./passthrough x1 | save output.csv",
    )
    .unwrap();

    let out = run_ril(&dir);
    assert!(
        out.status.success(),
        "ril failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );

    let csv = fs::read_to_string(dir.path().join("output.csv")).expect("output.csv not created");
    assert!(csv.contains("x") && csv.contains("y"), "headers missing: {csv}");
    assert!(
        csv.contains("10") && csv.contains("20") && csv.contains("30") && csv.contains("40"),
        "data missing: {csv}"
    );
}

#[test]
fn test_bad_rilfile_syntax_exits_nonzero() {
    let dir = make_dir();
    fs::write(dir.path().join("rilfile"), "@@@invalid@@@").unwrap();

    let out = run_ril(&dir);
    assert!(
        !out.status.success(),
        "expected non-zero exit for invalid rilfile syntax"
    );

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("expected pipeline") || stderr.contains("1:1"),
        "expected a parse diagnostic in stderr\nstderr: {stderr:?}"
    );
}

#[test]
fn test_python_exception_exits_nonzero() {
    let dir = make_dir();
    fs::write(dir.path().join("input.csv"), "a,b\n1,2\n").unwrap();

    fs::write(
        dir.path().join("crash.py"),
        r#"from ril import rilfn

@rilfn
def process(batch):
    raise RuntimeError("intentional test error")
"#,
    )
    .unwrap();

    fs::write(
        dir.path().join("rilfile"),
        "load input.csv | crash.py x1 | save output.csv",
    )
    .unwrap();

    let out = run_ril(&dir);
    assert!(
        !out.status.success(),
        "expected non-zero exit when Python stage raises"
    );

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("RuntimeError") || stderr.contains("intentional"),
        "expected Python error details in stderr\nstderr: {stderr:?}"
    );
}

#[test]
fn test_schema_drift_exits_nonzero() {
    let dir = make_dir();
    fs::write(dir.path().join("input.csv"), "id\n1\n2\n3\n").unwrap();

    fs::write(
        dir.path().join("drift.py"),
        r#"from ril import rilfn
import pyarrow as pa

@rilfn
def process(batch):
    data = pa.record_batch(batch).to_pydict()
    if 2 in data['id']:
        data['extra'] = [0] * len(data['id'])
    return pa.RecordBatch.from_pydict(data)
"#,
    )
    .unwrap();

    fs::write(
        dir.path().join("rilfile"),
        "load input.csv +1 | drift.py x1 | save output.csv",
    )
    .unwrap();

    let out = run_ril(&dir);
    assert!(
        !out.status.success(),
        "expected non-zero exit when a stage's output schema changes between batches"
    );

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("output schema changed"),
        "expected a schema-mismatch diagnostic in stderr\nstderr: {stderr:?}"
    );
}
