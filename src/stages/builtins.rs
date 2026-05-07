use crate::stages::{BuiltinStage, ScriptStage, Stage};
use crate::stages::exec::{run_load, run_save, run_script};

pub fn run_stage(stage: Stage) -> anyhow::Result<()> {
    match stage {
        Stage::Builtin(BuiltinStage::Load{path}) => 
            run_load::run_load(&path),
        Stage::Builtin(BuiltinStage::Save{path}) => 
            run_save::run_save(&path),
        Stage::Script(ScriptStage{path, flags}) => 
            run_script::run_script(&path, flags),
        _ => Err(anyhow::anyhow!("Stage not implemented"))
    }
}