pub mod builtins;
mod exec;
pub mod script_manager;
pub mod worker;
pub mod manager;

#[derive(Clone)]
pub enum Stage {
    Builtin(BuiltinStage),
    Script(ScriptStage),
}

#[derive(Clone)]
pub enum BuiltinStage {
    Load { path: String, batch_size: usize },
    Save { path: String },
    Tee { path: String },
}

#[derive(Clone)]
pub enum WorkerMode {
    Default,
    Fixed(usize),
}

#[derive(Clone)]
pub struct ScriptStage {
    pub path: String,
    pub flags: Vec<(String, String)>,
    pub workers: WorkerMode,
}