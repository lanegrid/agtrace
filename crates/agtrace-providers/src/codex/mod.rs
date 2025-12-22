pub mod io;
pub mod mapper;
pub mod models;
pub mod parser;
pub mod schema;
pub mod tool_mapping;
pub mod tools;

use crate::traits::{ProbeResult, SessionIndex};
use crate::{ImportContext, LogProvider, ScanContext, SessionMetadata};
use agtrace_types::paths_equal;
use agtrace_types::AgentEvent;
use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub use self::io::{
    extract_codex_header, extract_cwd_from_codex_file, is_empty_codex_session, normalize_codex_file,
};
pub use self::mapper::CodexToolMapper;
pub use self::parser::CodexParser;

// --- New trait-based architecture ---

/// Codex discovery and lifecycle management
pub struct CodexDiscovery;

impl crate::traits::LogDiscovery for CodexDiscovery {
    fn id(&self) -> &'static str {
        "codex"
    }

    fn probe(&self, path: &Path) -> ProbeResult {
        if !path.is_file() {
            return ProbeResult::NoMatch;
        }

        // Skip empty files
        if let Ok(metadata) = std::fs::metadata(path) {
            if metadata.len() == 0 {
                return ProbeResult::NoMatch;
            }
        }

        let is_jsonl = path.extension().is_some_and(|e| e == "jsonl");
        let filename = path.file_name().and_then(|f| f.to_str()).unwrap_or("");

        if is_jsonl && filename.starts_with("rollout-") && !is_empty_codex_session(path) {
            ProbeResult::match_high()
        } else {
            ProbeResult::NoMatch
        }
    }

    fn resolve_log_root(&self, _project_root: &Path) -> Option<PathBuf> {
        // Codex doesn't organize by project hash
        None
    }

    fn scan_sessions(&self, log_root: &Path) -> Result<Vec<SessionIndex>> {
        let mut sessions: HashMap<String, SessionIndex> = HashMap::new();

        if !log_root.exists() {
            return Ok(Vec::new());
        }

        for entry in WalkDir::new(log_root).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();

            if self.probe(path) == ProbeResult::NoMatch {
                continue;
            }

            let header = match extract_codex_header(path) {
                Ok(h) => h,
                Err(_) => continue,
            };

            let session_id = match header.session_id {
                Some(id) => id,
                None => continue,
            };

            sessions
                .entry(session_id.clone())
                .or_insert_with(|| SessionIndex {
                    session_id: session_id.clone(),
                    timestamp: header.timestamp.clone(),
                    main_file: path.to_path_buf(),
                    sidechain_files: Vec::new(), // Codex doesn't have sidechains
                    project_root: header.cwd.clone(),
                    snippet: header.snippet.clone(),
                });
        }

        Ok(sessions.into_values().collect())
    }

    fn extract_session_id(&self, path: &Path) -> Result<String> {
        let header = extract_codex_header(path)?;
        header
            .session_id
            .ok_or_else(|| anyhow::anyhow!("No session_id in file: {}", path.display()))
    }

    fn find_session_files(&self, log_root: &Path, session_id: &str) -> Result<Vec<PathBuf>> {
        let mut matching_files = Vec::new();

        for entry in WalkDir::new(log_root).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();

            if self.probe(path) == ProbeResult::NoMatch {
                continue;
            }

            if let Ok(header) = extract_codex_header(path) {
                if header.session_id.as_deref() == Some(session_id) {
                    matching_files.push(path.to_path_buf());
                }
            }
        }

        Ok(matching_files)
    }
}

// --- Backward-compatible provider (Facade pattern) ---

/// Backward-compatible facade that delegates to new trait-based architecture
pub struct CodexProvider {
    adapter: crate::traits::ProviderAdapter,
}

impl Default for CodexProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl CodexProvider {
    pub fn new() -> Self {
        Self {
            adapter: crate::traits::ProviderAdapter::codex(),
        }
    }
}

impl LogProvider for CodexProvider {
    fn name(&self) -> &str {
        self.adapter.discovery.id()
    }

    fn can_handle(&self, path: &Path) -> bool {
        self.adapter.discovery.probe(path).is_match()
    }

    fn normalize_file(&self, path: &Path, _context: &ImportContext) -> Result<Vec<AgentEvent>> {
        self.adapter.parser.parse_file(path)
    }

    fn belongs_to_project(&self, path: &Path, target_project_root: &Path) -> bool {
        extract_cwd_from_codex_file(path)
            .map(|cwd| paths_equal(target_project_root, Path::new(&cwd)))
            .unwrap_or(false)
    }

    fn scan(&self, log_root: &Path, context: &ScanContext) -> Result<Vec<SessionMetadata>> {
        // Delegate to adapter's scan_legacy which uses the new discovery architecture
        self.adapter.scan_legacy(log_root, context)
    }

    fn find_session_files(&self, log_root: &Path, session_id: &str) -> Result<Vec<PathBuf>> {
        self.adapter
            .discovery
            .find_session_files(log_root, session_id)
    }

    fn extract_session_id(&self, path: &Path) -> Result<String> {
        self.adapter.discovery.extract_session_id(path)
    }

    fn classify_tool(
        &self,
        tool_name: &str,
    ) -> Option<(agtrace_types::ToolOrigin, agtrace_types::ToolKind)> {
        // Use adapter's mapper for classification
        let (origin, kind) = self.adapter.mapper.classify(tool_name);
        Some((origin, kind))
    }

    fn extract_summary(
        &self,
        _tool_name: &str,
        kind: agtrace_types::ToolKind,
        arguments: &serde_json::Value,
    ) -> Option<String> {
        // Use adapter's mapper for summary extraction
        Some(self.adapter.mapper.summarize(kind, arguments))
    }
}
