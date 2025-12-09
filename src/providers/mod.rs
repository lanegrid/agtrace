use crate::model::AgentEventV1;
use anyhow::Result;
use std::path::{Path, PathBuf};

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

    fn belongs_to_project(&self, path: &Path, target_project_root: &Path) -> bool;

    fn get_search_root(&self, _log_root: &Path, _target_project_root: &Path) -> Option<PathBuf> {
        None
    }
}
