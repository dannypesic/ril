pub mod builtins;
mod exec;

pub enum Stage {
    Builtin(BuiltinStage),
    Script(ScriptStage),
}

pub enum BuiltinStage {
    Load { path: String },
    Save { path: String },
    Filter { expr: String },
    Select { fields: Vec<String> },
    Each { inner: Box<Stage> },
}

pub struct ScriptStage {
    pub path: String,
    pub flags: Vec<(String, String)>,
}