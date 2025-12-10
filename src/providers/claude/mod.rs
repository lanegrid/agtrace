pub mod io;
pub mod mapper;

use crate::model::AgentEventV1;
use crate::providers::{ImportContext, LogProvider, LogFileMetadata, ScanContext, SessionMetadata};
use crate::utils::{encode_claude_project_dir, paths_equal};
use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub use self::io::{extract_cwd_from_claude_file, extract_claude_header, normalize_claude_file};

pub struct ClaudeProvider;

impl ClaudeProvider {
    pub fn new() -> Self {
        Self
    }
}

impl LogProvider for ClaudeProvider {
    fn name(&self) -> &str {
        "claude"
    }

    fn can_handle(&self, path: &Path) -> bool {
        path.is_file() && path.extension().map_or(false, |e| e == "jsonl")
    }

    fn normalize_file(&self, path: &Path, context: &ImportContext) -> Result<Vec<AgentEventV1>> {
        normalize_claude_file(path, context.project_root_override.as_deref())
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
            if !path.is_file() {
                continue;
            }

            if let Some(ext) = path.extension() {
                if ext != "jsonl" {
                    continue;
                }
            } else {
                continue;
            }

            let header = match extract_claude_header(path) {
                Ok(h) => h,
                Err(e) => {
                    eprintln!("Warning: Failed to parse header from {}: {}", path.display(), e);
                    continue;
                }
            };

            let session_id = match header.session_id {
                Some(id) => id,
                None => {
                    eprintln!("Warning: No session_id found in {}", path.display());
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
            }

            let metadata = std::fs::metadata(path).ok();
            let file_size = metadata.as_ref().map(|m| m.len() as i64);
            let mod_time = metadata.and_then(|m| m.modified().ok())
                .map(|t| format!("{:?}", t));

            let log_file = LogFileMetadata {
                path: path.display().to_string(),
                role: if path.file_name().and_then(|n| n.to_str()).map_or(false, |n| n.starts_with("agent-")) {
                    "sidechain".to_string()
                } else {
                    "main".to_string()
                },
                file_size,
                mod_time,
            };

            let session = sessions.entry(session_id.clone()).or_insert_with(|| SessionMetadata {
                session_id: session_id.clone(),
                project_hash: context.project_hash.clone(),
                project_root: header.cwd.clone(),
                provider: "claude".to_string(),
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
}
