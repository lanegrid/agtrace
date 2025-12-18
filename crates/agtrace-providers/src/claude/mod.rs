pub mod io;
pub mod models;
pub mod normalize;
pub mod schema;

use crate::{ImportContext, LogFileMetadata, LogProvider, ScanContext, SessionMetadata};
use agtrace_types::paths_equal;
use agtrace_types::AgentEvent;
use anyhow::Result;
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
}
