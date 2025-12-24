use crate::traits::{LogDiscovery, ProbeResult, SessionIndex};
use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use super::io::extract_claude_header;

pub struct ClaudeDiscovery;

impl LogDiscovery for ClaudeDiscovery {
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

        if let Ok(metadata) = std::fs::metadata(path) {
            if metadata.len() == 0 {
                return ProbeResult::NoMatch;
            }
        }

        ProbeResult::match_high()
    }

    fn resolve_log_root(&self, _project_root: &Path) -> Option<PathBuf> {
        None
    }

    fn scan_sessions(&self, log_root: &Path) -> Result<Vec<SessionIndex>> {
        let mut sessions: HashMap<String, SessionIndex> = HashMap::new();

        for entry in WalkDir::new(log_root)
            .max_depth(2)
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
                    project_root: header.cwd.clone(),
                    snippet: header.snippet.clone(),
                });

            if header.is_sidechain {
                if !session.sidechain_files.contains(&path.to_path_buf()) {
                    session.sidechain_files.push(path.to_path_buf());
                }
            } else {
                session.main_file = path.to_path_buf();
            }

            if !header.is_sidechain || session.timestamp.is_none() {
                if session.timestamp.is_none() {
                    session.timestamp = header.timestamp.clone();
                }
                if session.project_root.is_none() {
                    session.project_root = header.cwd.clone();
                }
                if session.snippet.is_none() {
                    session.snippet = header.snippet.clone();
                }
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
