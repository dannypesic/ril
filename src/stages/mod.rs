pub mod builtins;
mod exec;
pub mod script_manager;

#[derive(Clone)]
pub enum Stage {
    Builtin(BuiltinStage),
    Script(ScriptStage),
}

#[derive(Clone)]
pub enum BuiltinStage {
    Load { path: String },
    Save { path: String },
    Tee { path: String },
}

#[derive(Clone)]
pub struct ScriptStage {
    pub path: String,
    pub flags: Vec<(String, String)>,
}