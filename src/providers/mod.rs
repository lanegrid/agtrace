use crate::model::AgentEventV1;
use anyhow::Result;
use std::path::Path;

pub mod claude;
pub mod codex;
pub mod gemini;

pub use claude::ClaudeProvider;
pub use codex::CodexProvider;
pub use gemini::GeminiProvider;

pub struct ImportContext {
    pub project_root_override: Option<String>,
    pub session_id_prefix: Option<String>,
    pub all_projects: bool,
}

pub trait LogProvider: Send + Sync {
    fn name(&self) -> &str;

    fn can_handle(&self, path: &Path) -> bool;

    fn normalize_file(&self, path: &Path, context: &ImportContext) -> Result<Vec<AgentEventV1>>;
}
