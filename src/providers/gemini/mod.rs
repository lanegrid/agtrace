pub mod io;
pub mod mapper;
pub mod schema;

use crate::model::AgentEventV1;
use crate::providers::{ImportContext, LogFileMetadata, LogProvider, ScanContext, SessionMetadata};
use crate::utils::{is_64_char_hex, project_hash_from_root};
use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub use self::io::{
    extract_gemini_header, extract_project_hash_from_gemini_file, normalize_gemini_file,
};

pub struct GeminiProvider;

impl Default for GeminiProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl GeminiProvider {
    pub fn new() -> Self {
        Self
    }
}

impl LogProvider for GeminiProvider {
    fn name(&self) -> &str {
        "gemini"
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

        let filename = path.file_name().and_then(|f| f.to_str()).unwrap_or("");
        filename == "logs.json" || (filename.starts_with("session-") && filename.ends_with(".json"))
    }

    fn normalize_file(&self, path: &Path, _context: &ImportContext) -> Result<Vec<AgentEventV1>> {
        normalize_gemini_file(path)
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

            let filename = path.file_name().and_then(|f| f.to_str()).unwrap_or("");

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
                    role: if filename == "logs.json" {
                        "meta"
                    } else {
                        "main"
                    }
                    .to_string(),
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
}
