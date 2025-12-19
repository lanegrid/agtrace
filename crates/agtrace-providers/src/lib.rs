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

// Tool analysis (classification and summary extraction)
pub mod tool_analyzer;

// Tool specification (shared type for tool registries)
pub(crate) mod tool_spec;

// Re-export provider types
pub use claude::ClaudeProvider;
pub use codex::CodexProvider;
pub use gemini::GeminiProvider;

// Re-export normalize functions (for tests and external use)
pub use claude::normalize_claude_file;
pub use codex::normalize_codex_file;
pub use gemini::normalize_gemini_file;

// Re-export registry functions for convenience
pub use registry::{
    create_all_providers, create_provider, detect_provider_from_path, get_all_providers,
    get_default_log_paths, get_provider_metadata, get_provider_names,
};

// Re-export tool analyzer functions for convenience
pub use tool_analyzer::{classify_common, extract_common_summary, truncate};

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

    /// Find all log files belonging to a specific session
    ///
    /// This method is used by the watch command to discover all files (main + sidechain)
    /// associated with a session ID. It performs a lightweight filesystem scan without
    /// full normalization.
    ///
    /// # Arguments
    /// * `log_root` - The provider's log root directory
    /// * `session_id` - The session ID to search for
    ///
    /// # Returns
    /// A vector of absolute file paths belonging to the session, or an error if the scan fails.
    ///
    /// # Performance
    /// This method is called repeatedly by the watch command (every 500ms poll cycle).
    /// Implementations should be optimized for speed:
    /// - Scan only relevant directories (not the entire filesystem)
    /// - Use metadata/filename checks, not full file parsing
    /// - Target: <10ms for typical project directories (~100 files)
    fn find_session_files(&self, log_root: &Path, session_id: &str) -> Result<Vec<PathBuf>>;

    /// Extract session ID from a log file
    ///
    /// This method extracts the session ID from a log file without full normalization.
    /// Used by the watch command to identify which session a file belongs to.
    ///
    /// # Arguments
    /// * `path` - Path to the log file
    ///
    /// # Returns
    /// The session ID, or an error if it cannot be extracted
    ///
    /// # Performance
    /// This method should only read the file header/metadata, not parse the entire file.
    /// Target: <1ms per file
    fn extract_session_id(&self, path: &Path) -> Result<String>;

    /// Parse a single line for streaming/watch mode
    ///
    /// Returns:
    /// - `Ok(Some(event))` if the line was successfully parsed
    /// - `Ok(None)` if the line is malformed or incomplete (non-fatal, skip silently)
    /// - `Err(e)` if a fatal error occurred that should stop processing
    ///
    /// Default implementation assumes JSONL format.
    ///
    /// # Provider-specific implementations
    ///
    /// Providers storing logs in raw formats (not JSONL) should override this method
    /// to parse their specific format. For example:
    /// - Codex: Parse raw JSON format and convert to AgentEvent
    /// - Gemini: Parse raw JSON format and convert to AgentEvent
    /// - Claude: Already uses JSONL, default implementation works
    ///
    /// # Current limitation
    ///
    /// The watch command currently only supports JSONL format. Provider-specific
    /// raw format support requires passing the provider instance to SessionWatcher,
    /// which is planned for future implementation.
    fn parse_line(&self, line: &str) -> Result<Option<AgentEvent>> {
        match serde_json::from_str::<AgentEvent>(line) {
            Ok(event) => Ok(Some(event)),
            Err(_) => Ok(None), // Silently skip malformed lines
        }
    }

    /// Classify tool by origin and semantic kind
    ///
    /// Returns None if this provider doesn't recognize the tool (fallback to common logic)
    fn classify_tool(
        &self,
        _tool_name: &str,
    ) -> Option<(agtrace_types::ToolOrigin, agtrace_types::ToolKind)> {
        None
    }

    /// Extract summary string for UI display from tool arguments
    ///
    /// Returns None if this provider doesn't have custom extraction logic (fallback to common logic)
    fn extract_summary(
        &self,
        _tool_name: &str,
        _kind: agtrace_types::ToolKind,
        _arguments: &serde_json::Value,
    ) -> Option<String> {
        None
    }
}
