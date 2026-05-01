use crate::buffer::{Sender, Receiver};
use crate::stages::{BuiltinStage, ScriptStage, Stage};
use crate::stages::exec::{run_load, run_save, run_script};

pub fn run_stage(stage: Stage, rx: Option<Receiver>, tx: Option<Sender>) -> anyhow::Result<()> {
    match stage {
        Stage::Builtin(BuiltinStage::Load{path}) => 
            run_load::run_load(&path, tx.expect("missing tx")),
        Stage::Builtin(BuiltinStage::Save{path}) => 
            run_save::run_save(&path, rx.expect("missin rx")),
        Stage::Script(ScriptStage{path, flags}) => 
            run_script::run_script(&path, flags, rx.expect("missin rx"), tx.expect("missin tx")),
        _ => Err(anyhow::anyhow!("Stage not implemented"))
    }
}