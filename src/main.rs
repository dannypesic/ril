mod stages;
mod pipeline;
mod cli;
pub mod setup;
pub mod error;

use crate::pipeline::parser;
use crate::pipeline::executor;
use crate::setup::setup_env;
use crate::stages::builtins;
use crate::cli::RunMode;

fn main() {
    if let Err(e) = run() {
        eprintln!("ril: {e}");
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        println!("ril: parallel Python pipeline executor\n");
        println!("Reads a `rilfile` in the current directory and runs the pipeline.\n");
        println!("Usage:");
        println!("  ril          run the pipeline defined in ./rilfile");
        println!("  ril -h       show this help\n");
        println!("Docs and examples: https://github.com/dannypesic/ril");
        return Ok(());
    }

    let mode = RunMode::detect()?;

    match mode {
        RunMode::Pipeline => {
            println!("Preparing environment, please wait...");
            setup_env()?;
            println!("Spinning the interpreters...");

            let pipeline_src = std::fs::read_to_string("rilfile")?;
            let stages = parser::parse(&pipeline_src)?;

            if let Err(_) = executor::run(stages) {
                eprint!("\r\x1b[2K");
                std::process::exit(1);
            }
        }
        RunMode::StageWorker { index, is_python } => {
            let pipeline_src = std::fs::read_to_string("rilfile")?;
            let stages = parser::parse(&pipeline_src)?;

            if let Err(e) = builtins::run_stage(stages[index].clone(), is_python, index) {
                let msg = e.to_string().replace('\n', "\\n");
                eprintln!("RIL_ERROR:{msg}");
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
