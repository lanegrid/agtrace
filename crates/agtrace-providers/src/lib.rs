use anyhow::Result;
use std::path::{Path, PathBuf};

// New trait-based architecture (public API)
pub mod traits;

// Re-export new trait-based types for public use
pub use traits::{
    LogDiscovery, ProbeResult, ProviderAdapter, SessionIndex, SessionParser, ToolMapper,
};

// Provider implementations (internal modules)
pub mod claude;
pub mod codex;
pub mod gemini;

// Event builder (shared normalization utility)
pub mod builder;

// Tool call normalization (provider-specific logic)
pub mod normalization;

// Provider registry
pub mod registry;

// Token limits resolution
pub mod token_limits;

// Tool analysis (classification and summary extraction)
pub mod tool_analyzer;

// Tool specification (shared type for tool registries)
pub(crate) mod tool_spec;

// Provider implementations are now internal; consumers use ProviderAdapter

// Re-export normalize functions (for tests and external use)
pub use claude::normalize_claude_file;
pub use codex::normalize_codex_file;
pub use gemini::normalize_gemini_file;

// Re-export registry functions for convenience
pub use registry::{
    create_adapter, create_all_adapters, detect_adapter_from_path, get_all_providers,
    get_default_log_paths, get_provider_metadata, get_provider_names,
};

// Re-export tool analyzer functions for convenience
pub use tool_analyzer::{classify_common, extract_common_summary, truncate};

// Re-export normalization functions for convenience
pub use normalization::normalize_tool_call;

// --- Legacy types kept for backward compatibility ---
// These are still used by consumers but may be phased out in future

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

pub struct ScanContext {
    pub project_hash: String,
    pub project_root: Option<String>,
}

// --- ProviderAdapter extensions for backward compatibility ---

impl ProviderAdapter {
    /// Legacy support: Scan sessions and convert to full SessionMetadata
    ///
    /// This bridges the gap between the new efficient SessionIndex and the old UI-heavy SessionMetadata.
    /// The new discovery layer provides lightweight SessionIndex, and this method enriches it with
    /// file metadata for backward compatibility with existing UI code.
    pub fn scan_legacy(
        &self,
        log_root: &Path,
        context: &ScanContext,
    ) -> Result<Vec<SessionMetadata>> {
        // 1. Use new efficient discovery
        let sessions = self.discovery.scan_sessions(log_root)?;

        let mut metadata_list = Vec::new();

        // 2. Filter and enrich for legacy UI
        for session in sessions {
            // Filter by project_root if specified (provider-specific filtering)
            if let Some(expected_root) = &context.project_root {
                // Claude and Codex: Match project_root from session
                // Gemini: Uses project_hash instead, so we accept all here
                // (Gemini's scan_sessions already filters by project_hash at discovery level)
                if let Some(session_root) = &session.project_root {
                    let session_normalized = session_root.trim_end_matches('/');
                    let expected_normalized = expected_root.trim_end_matches('/');
                    if session_normalized != expected_normalized {
                        continue;
                    }
                }
                // If session doesn't have project_root but context expects one, skip it
                // (unless this is Gemini which handles project filtering differently)
                else if self.id() != "gemini" {
                    continue;
                }
            }

            let mut log_files = Vec::new();

            // Helper to convert file path to LogFileMetadata
            let to_log_file = |path: &PathBuf, role: &str| -> LogFileMetadata {
                let meta = std::fs::metadata(path).ok();
                LogFileMetadata {
                    path: path.display().to_string(),
                    role: role.to_string(),
                    file_size: meta.as_ref().map(|m| m.len() as i64),
                    mod_time: meta
                        .and_then(|m| m.modified().ok())
                        .map(|t| format!("{:?}", t)),
                }
            };

            // Add main file
            log_files.push(to_log_file(&session.main_file, "main"));

            // Add sidechain files
            for side_file in &session.sidechain_files {
                log_files.push(to_log_file(side_file, "sidechain"));
            }

            metadata_list.push(SessionMetadata {
                session_id: session.session_id,
                project_hash: context.project_hash.clone(),
                project_root: session.project_root,
                provider: self.id().to_string(),
                start_ts: session.timestamp,
                end_ts: None,
                snippet: session.snippet,
                log_files,
            });
        }

        Ok(metadata_list)
    }
}
