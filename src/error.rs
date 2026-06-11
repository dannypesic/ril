use std::fmt;

#[derive(Debug)]
pub enum RilError {
    Parse(String),
    Stage { index: usize, name: String, inner: String },
    Worker { worker_index: usize, batch_index: usize, inner: String },
    Python { traceback: String },
    Fatal(String),
}

impl fmt::Display for RilError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RilError::Parse(msg) =>
                write!(f, "parse error: {msg}"),
            RilError::Stage { index, name, inner } =>
                write!(f, "stage[{index}] `{name}` failed: {inner}"),
            RilError::Worker { worker_index, batch_index, inner } =>
                write!(f, "worker[{worker_index}] at batch {batch_index}: {inner}"),
            RilError::Python { traceback } =>
                write!(f, "python error:\n{traceback}"),
            RilError::Fatal(msg) =>
                write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for RilError {}

pub const ERROR_PREFIX: &str = "RIL_ERROR:";
