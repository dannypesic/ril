mod stages;
mod pipeline;
pub mod setup;

use clap::Parser;
use crate::pipeline::parser;
use crate::pipeline::executor;
use crate::setup::setup_env;
use crate::stages::{builtins};

#[derive(Parser)]
struct Args {
    #[arg(long)]
    stage_index: Option<usize>,
    #[arg(long)]
    worker_is_python: Option<bool>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    if args.stage_index.is_none() {
        println!("Preparing environment, please wait...");
        setup_env()?;
        println!("Spinning the interpreters...");
    }

    let pipeline_src = std::fs::read_to_string(&"rilfile")?;
    let stages = parser::parse(&pipeline_src)?;

    if let Some(index) = args.stage_index {
        let is_python = args.worker_is_python.unwrap_or(false);
        builtins::run_stage(stages[index].clone(), is_python, index).expect("Stage failed to run");
    }
    else {
        executor::run(stages)?;
    }

    Ok(())
}