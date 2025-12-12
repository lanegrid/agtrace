use agtrace_types::AgentEventV1;
use anyhow::Result;
use std::path::{Path, PathBuf};

// Provider implementations (internal modules)
mod claude;
mod codex;
mod gemini;

// Re-export provider types
pub use claude::ClaudeProvider;
pub use codex::CodexProvider;
pub use gemini::GeminiProvider;

// Re-export normalize functions (for tests and external use)
pub use claude::normalize_claude_file;
pub use codex::normalize_codex_file;
pub use gemini::normalize_gemini_file;

#[derive(Debug, Clone)]
pub struct LogFileMetadata {
    pub path: String,
    pub role: String,
    pub file_size: Option<i64>,
    pub mod_time: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SessionMetadata {
    pub session_id: String,
    pub project_hash: String,
    pub project_root: Option<String>,
    pub provider: String,
    pub start_ts: Option<String>,
    pub end_ts: Option<String>,
    pub snippet: Option<String>,
    pub log_files: Vec<LogFileMetadata>,
}

pub struct ImportContext {
    pub project_root_override: Option<String>,
    pub session_id_prefix: Option<String>,
    pub all_projects: bool,
}

pub struct ScanContext {
    pub project_hash: String,
    pub project_root: Option<String>,
}

pub trait LogProvider: Send + Sync {
    fn name(&self) -> &str;

    fn can_handle(&self, path: &Path) -> bool;

    fn normalize_file(&self, path: &Path, context: &ImportContext) -> Result<Vec<AgentEventV1>>;

    fn belongs_to_project(&self, path: &Path, target_project_root: &Path) -> bool;

    fn get_search_root(&self, _log_root: &Path, _target_project_root: &Path) -> Option<PathBuf> {
        None
    }

    fn scan(&self, log_root: &Path, context: &ScanContext) -> Result<Vec<SessionMetadata>>;
}
