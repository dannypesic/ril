use std::env;

#[derive(Debug, Clone)]
pub enum RunMode {
    Pipeline,
    StageWorker { index: usize, is_python: bool },
}

impl RunMode {
    pub fn detect() -> anyhow::Result<Self> {
        if let Ok(index_str) = env::var("RIL_STAGE_INDEX") {
            let index = index_str.parse()?;
            let is_python = env::var("RIL_WORKER_IS_PYTHON")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(false);

            Ok(RunMode::StageWorker { index, is_python })
        } else {
            Ok(RunMode::Pipeline)
        }
    }
}
