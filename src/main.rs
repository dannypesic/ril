mod stages;
mod pipeline;
mod cli;
pub mod setup;

use crate::pipeline::parser;
use crate::pipeline::executor;
use crate::setup::setup_env;
use crate::stages::builtins;
use crate::cli::RunMode;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        println!("help");
        return Ok(());
    }

    let mode = RunMode::detect()?;

    match mode {
        RunMode::Pipeline => {
            println!("Preparing environment, please wait...");
            setup_env()?;
            println!("Spinning the interpreters...");

            let pipeline_src = std::fs::read_to_string(&"rilfile")?;
            let stages = parser::parse(&pipeline_src)?;
            executor::run(stages)?;
        }
        RunMode::StageWorker { index, is_python } => {
            let pipeline_src = std::fs::read_to_string(&"rilfile")?;
            let stages = parser::parse(&pipeline_src)?;
            builtins::run_stage(stages[index].clone(), is_python, index)?;
        }
    }

    Ok(())
}