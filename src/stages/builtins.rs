use crate::stages::{BuiltinStage, ScriptStage, Stage};
use crate::stages::exec::{run_load, run_save, run_script, run_tee};

pub fn run_stage(stage: Stage) -> anyhow::Result<()> {
    match stage {
        Stage::Builtin(BuiltinStage::Load{path}) => 
            run_load::run_load(&path),
        Stage::Builtin(BuiltinStage::Save{path}) => 
            run_save::run_save(&path),
        Stage::Builtin(BuiltinStage::Tee{path}) =>
            run_tee::run_tee(&path),
        Stage::Script(ScriptStage{path, flags}) => 
            run_script::run_script(&path, flags),
    }
}