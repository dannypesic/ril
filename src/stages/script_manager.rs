use crate::stages::Stage;
use crate::stages::builtins::run_stage;

/// Later on, this will provide:
/// Error handling,
/// Multiprocessing (with dynamic allocation),
/// Exit codes,
/// Status updates to parent

pub fn run_stage_process(stage: Stage) -> anyhow::Result<()> {
    run_stage(stage)
}

