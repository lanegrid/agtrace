pub mod io;
pub mod models;
pub mod normalize;
pub mod schema;
pub mod tool_mapping;
pub mod tools;

use crate::traits::{ProbeResult, SessionIndex};
use crate::{ImportContext, LogFileMetadata, LogProvider, ScanContext, SessionMetadata};
use agtrace_types::AgentEvent;
use agtrace_types::{
    is_64_char_hex, project_hash_from_root, ToolCallPayload, ToolKind, ToolOrigin,
};
use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub use self::io::{
    extract_gemini_header, extract_project_hash_from_gemini_file, normalize_gemini_file,
};

// --- New trait-based architecture ---

/// Gemini discovery and lifecycle management
pub struct GeminiDiscovery;

impl crate::traits::LogDiscovery for GeminiDiscovery {
    fn id(&self) -> &'static str {
        "gemini"
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

        let filename = path.file_name().and_then(|f| f.to_str()).unwrap_or("");
        // Only handle session-*.json files
        if filename.starts_with("session-") && filename.ends_with(".json") {
            ProbeResult::match_high()
        } else {
            ProbeResult::NoMatch
        }
    }

    fn resolve_log_root(&self, project_root: &Path) -> Option<PathBuf> {
        let hash = project_hash_from_root(&project_root.to_string_lossy());
        Some(PathBuf::from(hash))
    }

    fn scan_sessions(&self, log_root: &Path) -> Result<Vec<SessionIndex>> {
        let mut sessions: HashMap<String, SessionIndex> = HashMap::new();

        if !log_root.exists() {
            return Ok(Vec::new());
        }

        for entry in WalkDir::new(log_root)
            .max_depth(3)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            if self.probe(path) == ProbeResult::NoMatch {
                continue;
            }

            let header = match extract_gemini_header(path) {
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
                    sidechain_files: Vec::new(), // Gemini doesn't have sidechains
                });
        }

        Ok(sessions.into_values().collect())
    }

    fn extract_session_id(&self, path: &Path) -> Result<String> {
        let header = extract_gemini_header(path)?;
        header
            .session_id
            .ok_or_else(|| anyhow::anyhow!("No session_id in file: {}", path.display()))
    }

    fn find_session_files(&self, log_root: &Path, session_id: &str) -> Result<Vec<PathBuf>> {
        let mut matching_files = Vec::new();

        for entry in WalkDir::new(log_root)
            .max_depth(3)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            if self.probe(path) == ProbeResult::NoMatch {
                continue;
            }

            if let Ok(header) = extract_gemini_header(path) {
                if header.session_id.as_deref() == Some(session_id) {
                    matching_files.push(path.to_path_buf());
                }
            }
        }

        Ok(matching_files)
    }
}

/// Gemini session parser
pub struct GeminiParser;

impl crate::traits::SessionParser for GeminiParser {
    fn parse_file(&self, path: &Path) -> Result<Vec<AgentEvent>> {
        normalize_gemini_file(path)
    }

    fn parse_record(&self, content: &str) -> Result<Option<AgentEvent>> {
        // Gemini uses JSON format, parse as AgentEvent
        match serde_json::from_str::<AgentEvent>(content) {
            Ok(event) => Ok(Some(event)),
            Err(_) => Ok(None), // Skip malformed lines
        }
    }
}

/// Gemini tool mapper
pub struct GeminiToolMapper;

impl crate::traits::ToolMapper for GeminiToolMapper {
    fn classify(&self, tool_name: &str) -> (ToolOrigin, ToolKind) {
        tool_mapping::classify_tool(tool_name)
            .unwrap_or_else(|| crate::tool_analyzer::classify_common(tool_name))
    }

    fn normalize_call(&self, name: &str, args: Value, call_id: Option<String>) -> ToolCallPayload {
        normalize::normalize_gemini_tool_call(name.to_string(), args, call_id)
    }

    fn summarize(&self, kind: ToolKind, args: &Value) -> String {
        crate::tool_analyzer::extract_common_summary(kind, args)
    }
}

// --- Backward-compatible provider (Facade pattern) ---

/// Backward-compatible facade that delegates to new trait-based architecture
pub struct GeminiProvider {
    adapter: crate::traits::ProviderAdapter,
}

impl Default for GeminiProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl GeminiProvider {
    pub fn new() -> Self {
        Self {
            adapter: crate::traits::ProviderAdapter::gemini(),
        }
    }
}

impl LogProvider for GeminiProvider {
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
        let target_hash = project_hash_from_root(&target_project_root.to_string_lossy());
        if let Some(file_hash) = extract_project_hash_from_gemini_file(path) {
            file_hash == target_hash
        } else {
            if let Some(parent) = path.parent() {
                if let Some(dir_name) = parent.file_name().and_then(|n| n.to_str()) {
                    if is_64_char_hex(dir_name) {
                        return dir_name == target_hash;
                    }
                }
            }
            false
        }
    }

    fn get_search_root(&self, log_root: &Path, target_project_root: &Path) -> Option<PathBuf> {
        let hash = project_hash_from_root(&target_project_root.to_string_lossy());
        let dir = log_root.join(&hash);
        (dir.exists() && dir.is_dir()).then_some(dir)
    }

    fn scan(&self, log_root: &Path, context: &ScanContext) -> Result<Vec<SessionMetadata>> {
        let mut sessions: HashMap<String, SessionMetadata> = HashMap::new();

        if !log_root.exists() {
            return Ok(Vec::new());
        }

        let target_dir = if context.project_root.is_some() {
            log_root.join(&context.project_hash)
        } else {
            log_root.to_path_buf()
        };

        if !target_dir.exists() && context.project_root.is_some() {
            return Ok(Vec::new());
        }

        let search_root = if target_dir.exists() && target_dir != log_root {
            target_dir
        } else {
            log_root.to_path_buf()
        };

        for entry in WalkDir::new(&search_root)
            .max_depth(3)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            // Use can_handle for consistent filtering (filename pattern + empty files)
            if !self.can_handle(path) {
                continue;
            }

            if let Some(parent) = path.parent() {
                if let Some(dir_name) = parent.file_name().and_then(|n| n.to_str()) {
                    if is_64_char_hex(dir_name)
                        && context.project_root.is_some()
                        && dir_name != context.project_hash
                    {
                        continue;
                    }
                }
            }

            let header = match extract_gemini_header(path) {
                Ok(h) => h,
                Err(_) => {
                    // Skip files that can't be parsed (e.g., corrupted files)
                    continue;
                }
            };

            if let Some(session_id) = header.session_id {
                let metadata = std::fs::metadata(path).ok();
                let file_size = metadata.as_ref().map(|m| m.len() as i64);
                let mod_time = metadata
                    .and_then(|m| m.modified().ok())
                    .map(|t| format!("{:?}", t));

                let log_file = LogFileMetadata {
                    path: path.display().to_string(),
                    role: "main".to_string(),
                    file_size,
                    mod_time,
                };

                let session =
                    sessions
                        .entry(session_id.clone())
                        .or_insert_with(|| SessionMetadata {
                            session_id: session_id.clone(),
                            project_hash: context.project_hash.clone(),
                            project_root: None,
                            provider: "gemini".to_string(),
                            start_ts: header.timestamp.clone(),
                            end_ts: None,
                            snippet: header.snippet.clone(),
                            log_files: Vec::new(),
                        });

                session.log_files.push(log_file);

                if session.start_ts.is_none() {
                    session.start_ts = header.timestamp.clone();
                }
                if session.snippet.is_none() {
                    session.snippet = header.snippet.clone();
                }
            }
        }

        Ok(sessions.into_values().collect())
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
