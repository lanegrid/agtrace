pub mod io;
pub mod mapper;
pub mod schema;

use crate::model::AgentEventV1;
use crate::providers::{ImportContext, LogFileMetadata, LogProvider, ScanContext, SessionMetadata};
use crate::utils::paths_equal;
use anyhow::Result;
use std::path::Path;
use walkdir::WalkDir;

pub use self::io::{
    extract_codex_header, extract_cwd_from_codex_file, is_empty_codex_session, normalize_codex_file,
};

pub struct CodexProvider;

impl CodexProvider {
    pub fn new() -> Self {
        Self
    }
}

impl LogProvider for CodexProvider {
    fn name(&self) -> &str {
        "codex"
    }

    fn can_handle(&self, path: &Path) -> bool {
        if !path.is_file() {
            return false;
        }

        // Skip empty files
        if let Ok(metadata) = std::fs::metadata(path) {
            if metadata.len() == 0 {
                return false;
            }
        }

        let is_jsonl = path.extension().map_or(false, |e| e == "jsonl");
        let filename = path.file_name().and_then(|f| f.to_str()).unwrap_or("");

        is_jsonl && filename.starts_with("rollout-") && !is_empty_codex_session(path)
    }

    fn normalize_file(&self, path: &Path, context: &ImportContext) -> Result<Vec<AgentEventV1>> {
        normalize_codex_file(path, context.project_root_override.as_deref())
    }

    fn belongs_to_project(&self, path: &Path, target_project_root: &Path) -> bool {
        extract_cwd_from_codex_file(path)
            .map(|cwd| paths_equal(target_project_root, Path::new(&cwd)))
            .unwrap_or(false)
    }

    fn scan(&self, log_root: &Path, context: &ScanContext) -> Result<Vec<SessionMetadata>> {
        let mut sessions: Vec<SessionMetadata> = Vec::new();

        if !log_root.exists() {
            return Ok(Vec::new());
        }

        for entry in WalkDir::new(log_root).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();

            // Use can_handle for consistent filtering (filename pattern + empty files + empty sessions)
            if !self.can_handle(path) {
                continue;
            }

            let header = match extract_codex_header(path) {
                Ok(h) => h,
                Err(_) => {
                    // Skip files that can't be parsed (e.g., corrupted files)
                    continue;
                }
            };

            let session_id = match header.session_id {
                Some(id) => id,
                None => {
                    // Skip files without session_id (e.g., incomplete sessions)
                    continue;
                }
            };

            if let Some(cwd) = &header.cwd {
                if let Some(expected) = &context.project_root {
                    let cwd_normalized = cwd.trim_end_matches('/');
                    let expected_normalized = expected.trim_end_matches('/');
                    if cwd_normalized != expected_normalized {
                        continue;
                    }
                }
            } else if context.project_root.is_some() {
                continue;
            }

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

            sessions.push(SessionMetadata {
                session_id,
                project_hash: context.project_hash.clone(),
                project_root: header.cwd,
                provider: "codex".to_string(),
                start_ts: header.timestamp,
                end_ts: None,
                snippet: header.snippet,
                log_files: vec![log_file],
            });
        }

        Ok(sessions)
    }
}
