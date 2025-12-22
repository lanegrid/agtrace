pub mod io;
pub mod models;
pub mod normalize;
pub mod schema;
pub mod tool_mapping;
pub mod tools;

use crate::traits::{ProbeResult, SessionIndex};
use crate::{ImportContext, LogFileMetadata, LogProvider, ScanContext, SessionMetadata};
use agtrace_types::AgentEvent;
use agtrace_types::{paths_equal, ToolCallPayload, ToolKind, ToolOrigin};
use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub use self::io::{extract_claude_header, extract_cwd_from_claude_file, normalize_claude_file};

/// Encode project_root path to Claude Code directory name format
/// Claude Code replaces both '/' and '.' with '-'
fn encode_claude_project_dir(project_root: &Path) -> String {
    let path_str = project_root.to_string_lossy();
    let encoded = path_str
        .replace(['/', '.'], "-")
        .trim_start_matches('-')
        .to_string();
    format!("-{}", encoded)
}

// --- New trait-based architecture ---

/// Claude discovery and lifecycle management
pub struct ClaudeDiscovery;

impl crate::traits::LogProvider for ClaudeDiscovery {
    fn id(&self) -> &'static str {
        "claude_code"
    }

    fn probe(&self, path: &Path) -> ProbeResult {
        if !path.is_file() {
            return ProbeResult::NoMatch;
        }

        if path.extension().is_none_or(|e| e != "jsonl") {
            return ProbeResult::NoMatch;
        }

        // Skip empty files
        if let Ok(metadata) = std::fs::metadata(path) {
            if metadata.len() == 0 {
                return ProbeResult::NoMatch;
            }
        }

        ProbeResult::match_high()
    }

    fn resolve_log_root(&self, _project_root: &Path) -> Option<PathBuf> {
        // Claude doesn't have a single log root per project
        // Files are organized under ~/.claude/projects/-encoded-project-name/
        None
    }

    fn scan_sessions(&self, log_root: &Path) -> Result<Vec<SessionIndex>> {
        let mut sessions: HashMap<String, SessionIndex> = HashMap::new();

        for entry in WalkDir::new(log_root)
            .max_depth(3)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            if self.probe(path) == ProbeResult::NoMatch {
                continue;
            }

            let header = match extract_claude_header(path) {
                Ok(h) => h,
                Err(_) => continue,
            };

            let session_id = match header.session_id {
                Some(id) => id,
                None => continue,
            };

            let session = sessions
                .entry(session_id.clone())
                .or_insert_with(|| SessionIndex {
                    session_id: session_id.clone(),
                    timestamp: header.timestamp.clone(),
                    main_file: path.to_path_buf(),
                    sidechain_files: Vec::new(),
                });

            if header.is_sidechain {
                session.sidechain_files.push(path.to_path_buf());
            }
        }

        Ok(sessions.into_values().collect())
    }

    fn extract_session_id(&self, path: &Path) -> Result<String> {
        let header = extract_claude_header(path)?;
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

            if let Ok(header) = extract_claude_header(path) {
                if header.session_id.as_deref() == Some(session_id) {
                    matching_files.push(path.to_path_buf());
                }
            }
        }

        Ok(matching_files)
    }
}

/// Claude session parser
pub struct ClaudeParser;

impl crate::traits::SessionParser for ClaudeParser {
    fn parse_file(&self, path: &Path) -> Result<Vec<AgentEvent>> {
        normalize_claude_file(path)
    }

    fn parse_record(&self, content: &str) -> Result<Option<AgentEvent>> {
        // Claude uses JSONL format, parse as AgentEvent
        match serde_json::from_str::<AgentEvent>(content) {
            Ok(event) => Ok(Some(event)),
            Err(_) => Ok(None), // Skip malformed lines
        }
    }
}

/// Claude tool mapper
pub struct ClaudeToolMapper;

impl crate::traits::ToolMapper for ClaudeToolMapper {
    fn classify(&self, tool_name: &str) -> (ToolOrigin, ToolKind) {
        tool_mapping::classify_tool(tool_name)
            .unwrap_or_else(|| crate::tool_analyzer::classify_common(tool_name))
    }

    fn normalize_call(&self, name: &str, args: Value, call_id: Option<String>) -> ToolCallPayload {
        normalize::normalize_claude_tool_call(name.to_string(), args, call_id)
    }

    fn summarize(&self, kind: ToolKind, args: &Value) -> String {
        crate::tool_analyzer::extract_common_summary(kind, args)
    }
}

// --- Backward-compatible provider ---

pub struct ClaudeProvider;

impl Default for ClaudeProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ClaudeProvider {
    pub fn new() -> Self {
        Self
    }
}

impl LogProvider for ClaudeProvider {
    fn name(&self) -> &str {
        "claude_code"
    }

    fn can_handle(&self, path: &Path) -> bool {
        if !path.is_file() {
            return false;
        }

        if path.extension().is_none_or(|e| e != "jsonl") {
            return false;
        }

        // Skip empty files
        if let Ok(metadata) = std::fs::metadata(path) {
            if metadata.len() == 0 {
                return false;
            }
        }

        true
    }

    fn normalize_file(&self, path: &Path, _context: &ImportContext) -> Result<Vec<AgentEvent>> {
        normalize_claude_file(path)
    }

    fn belongs_to_project(&self, path: &Path, target_project_root: &Path) -> bool {
        extract_cwd_from_claude_file(path)
            .map(|cwd| paths_equal(target_project_root, Path::new(&cwd)))
            .unwrap_or(false)
    }

    fn get_search_root(&self, log_root: &Path, target_project_root: &Path) -> Option<PathBuf> {
        let dir_name = encode_claude_project_dir(target_project_root);
        let project_specific_root = log_root.join(dir_name);
        (project_specific_root.exists() && project_specific_root.is_dir())
            .then_some(project_specific_root)
    }

    fn scan(&self, log_root: &Path, context: &ScanContext) -> Result<Vec<SessionMetadata>> {
        let mut sessions: HashMap<String, SessionMetadata> = HashMap::new();

        let target_dir = if let Some(root) = &context.project_root {
            let encoded = encode_claude_project_dir(Path::new(root));
            log_root.join(&encoded)
        } else {
            log_root.to_path_buf()
        };

        if !target_dir.exists() {
            return Ok(Vec::new());
        }

        for entry in WalkDir::new(&target_dir)
            .max_depth(2)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            // Use can_handle for consistent filtering (extension + empty files)
            if !self.can_handle(path) {
                continue;
            }

            let header = match extract_claude_header(path) {
                Ok(h) => h,
                Err(_) => {
                    // Skip files that can't be parsed (e.g., corrupted files)
                    continue;
                }
            };

            let session_id = match header.session_id {
                Some(id) => id,
                None => {
                    // Skip files without session_id (e.g., metadata-only files)
                    continue;
                }
            };

            // Filter by project root if specified (exact match required)
            // Subdirectories are treated as completely separate projects to maintain
            // consistency with project_hash-based providers (Gemini)
            if let Some(cwd) = &header.cwd {
                if let Some(expected) = &context.project_root {
                    let cwd_normalized = cwd.trim_end_matches('/');
                    let expected_normalized = expected.trim_end_matches('/');
                    if cwd_normalized != expected_normalized {
                        continue;
                    }
                }
            }

            let metadata = std::fs::metadata(path).ok();
            let file_size = metadata.as_ref().map(|m| m.len() as i64);
            let mod_time = metadata
                .and_then(|m| m.modified().ok())
                .map(|t| format!("{:?}", t));

            let log_file = LogFileMetadata {
                path: path.display().to_string(),
                role: if header.is_sidechain {
                    "sidechain"
                } else {
                    "main"
                }
                .to_string(),
                file_size,
                mod_time,
            };

            let session = sessions
                .entry(session_id.clone())
                .or_insert_with(|| SessionMetadata {
                    session_id: session_id.clone(),
                    project_hash: context.project_hash.clone(),
                    project_root: header.cwd.clone(),
                    provider: "claude_code".to_string(),
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

        Ok(sessions.into_values().collect())
    }

    fn find_session_files(&self, log_root: &Path, session_id: &str) -> Result<Vec<PathBuf>> {
        let mut matching_files = Vec::new();

        // Claude stores files in encoded project directories
        // We need to scan all project directories since we don't know which one contains this session
        // Performance: Typical ~10ms for 100 files across multiple project directories
        for entry in WalkDir::new(log_root)
            .max_depth(3) // -encoded-project-dir/*.jsonl or -encoded-project-dir/subdir/*.jsonl
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            // Quick filter: must be a .jsonl file
            if !self.can_handle(path) {
                continue;
            }

            // Extract session_id from file header (lightweight check)
            if let Ok(header) = extract_claude_header(path) {
                if header.session_id.as_deref() == Some(session_id) {
                    matching_files.push(path.to_path_buf());
                }
            }
        }

        Ok(matching_files)
    }

    fn extract_session_id(&self, path: &Path) -> Result<String> {
        let header = extract_claude_header(path)?;
        header
            .session_id
            .ok_or_else(|| anyhow::anyhow!("No session_id in file: {}", path.display()))
    }

    fn classify_tool(
        &self,
        tool_name: &str,
    ) -> Option<(agtrace_types::ToolOrigin, agtrace_types::ToolKind)> {
        tool_mapping::classify_tool(tool_name)
    }

    fn extract_summary(
        &self,
        tool_name: &str,
        kind: agtrace_types::ToolKind,
        arguments: &serde_json::Value,
    ) -> Option<String> {
        tool_mapping::extract_summary(tool_name, kind, arguments)
    }
}
