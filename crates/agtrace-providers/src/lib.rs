use agtrace_types::AgentEvent;
use anyhow::Result;
use std::path::{Path, PathBuf};

// Provider implementations (internal modules)
pub mod claude;
pub mod codex;
pub mod gemini;

// Event builder (shared normalization utility)
pub mod builder;

// Provider registry
pub mod registry;

// Token limits resolution
pub mod token_limits;

// Re-export provider types
pub use claude::ClaudeProvider;
pub use codex::CodexProvider;
pub use gemini::GeminiProvider;

// Re-export v2 normalize functions (for tests and external use)
pub use claude::normalize_claude_file_v2;
pub use codex::normalize_codex_file_v2;
pub use gemini::normalize_gemini_file_v2;

// Re-export registry functions for convenience
pub use registry::{
    create_all_providers, create_provider, detect_provider_from_path, get_all_providers,
    get_default_log_paths, get_provider_metadata, get_provider_names,
};

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

    fn normalize_file(&self, path: &Path, context: &ImportContext) -> Result<Vec<AgentEvent>>;

    fn belongs_to_project(&self, path: &Path, target_project_root: &Path) -> bool;

    fn get_search_root(&self, _log_root: &Path, _target_project_root: &Path) -> Option<PathBuf> {
        None
    }

    fn scan(&self, log_root: &Path, context: &ScanContext) -> Result<Vec<SessionMetadata>>;

    /// Parse a single line for streaming/watch mode
    ///
    /// Returns:
    /// - `Ok(Some(event))` if the line was successfully parsed
    /// - `Ok(None)` if the line is malformed or incomplete (non-fatal, skip silently)
    /// - `Err(e)` if a fatal error occurred that should stop processing
    ///
    /// Default implementation assumes v2 schema JSONL format.
    ///
    /// # Provider-specific implementations
    ///
    /// Providers storing logs in raw formats (not v2 JSONL) should override this method
    /// to parse their specific format. For example:
    /// - Codex: Parse raw JSON format and convert to AgentEvent
    /// - Gemini: Parse raw JSON format and convert to AgentEvent
    /// - Claude: Already uses v2 JSONL, default implementation works
    ///
    /// # Current limitation
    ///
    /// The watch command currently only supports v2 JSONL format. Provider-specific
    /// raw format support requires passing the provider instance to SessionWatcher,
    /// which is planned for future implementation.
    fn parse_line(&self, line: &str) -> Result<Option<AgentEvent>> {
        match serde_json::from_str::<AgentEvent>(line) {
            Ok(event) => Ok(Some(event)),
            Err(_) => Ok(None), // Silently skip malformed lines
        }
    }
}
