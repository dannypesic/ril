mod stages;
mod pipeline;

use clap::Parser;
use crate::pipeline::parser;
use crate::pipeline::executor;
use crate::stages::script_manager;

#[derive(Parser)]
struct Args {
    #[arg(long)]
    stage_index: Option<usize>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let pipeline_src = std::fs::read_to_string(&"rilfile")?;
    let stages = parser::parse(&pipeline_src)?;

    if let Some(index) = args.stage_index {
        script_manager::run_stage_process(stages[index].clone()).expect("Stage failed to run");
    } else {
        executor::run(stages)?;
    }

    Ok(())
}