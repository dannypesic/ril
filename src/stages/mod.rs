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
    Filter { expr: String },                // For later
    Select { fields: Vec<String> },         // For later
    Each { inner: Box<Stage> },             // For later
}

#[derive(Clone)]
pub struct ScriptStage {
    pub path: String,
    pub flags: Vec<(String, String)>,
}