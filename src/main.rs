mod stages;
mod pipeline;
mod buffer;

use clap::Parser;
use crate::pipeline::parser;
use crate::pipeline::executor;

#[derive(Parser)] // ignore

struct Args {
    /// Pipeline string, e.g. "load data.csv | clean.py | save out.csv"
    /// Or a path to a .ril file
    pipeline: String,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let pipeline_src = if args.pipeline.ends_with(".ril") {
        std::fs::read_to_string(&args.pipeline)?
    } else {
        args.pipeline
    };

    let stages = parser::parse(&pipeline_src)?;
    println!("{:?}", &pipeline_src);
    println!("Cardinality {:?}", &stages.len());
    for s in &stages {                                                                                                                                                                                        
        match s {                                                                                                                                                                                             
            crate::stages::Stage::Builtin(_) => eprintln!("Builtin"),                                                                                                                                     
            crate::stages::Stage::Script(sc) => eprintln!("Script: {}", sc.path),                                                                                                                         
        }
    }
    executor::run(stages)?;

    Ok(())
}