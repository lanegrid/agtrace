use crate::traits::{LogDiscovery, ProbeResult, SessionIndex};
use agtrace_types::project_hash_from_root;
use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir::WalkDir;

use super::io::extract_gemini_header;

/// NOTE: Helper to get latest modification time from a list of files
/// Used to track when a session was last active (most recent file write)
fn get_latest_mod_time(files: &[&Path]) -> Option<String> {
    let mut latest: Option<SystemTime> = None;

    for path in files {
        if let Ok(metadata) = std::fs::metadata(path)
            && let Ok(modified) = metadata.modified()
            && (latest.is_none() || Some(modified) > latest)
        {
            latest = Some(modified);
        }
    }

    latest.map(|t| format!("{:?}", t))
}

pub struct GeminiDiscovery;

impl LogDiscovery for GeminiDiscovery {
    fn id(&self) -> &'static str {
        "gemini"
    }

    fn probe(&self, path: &Path) -> ProbeResult {
        if !path.is_file() {
            return ProbeResult::NoMatch;
        }

        if let Ok(metadata) = std::fs::metadata(path)
            && metadata.len() == 0
        {
            return ProbeResult::NoMatch;
        }

        let filename = path.file_name().and_then(|f| f.to_str()).unwrap_or("");
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
                    latest_mod_time: None, // Will be computed after all files are collected
                    main_file: path.to_path_buf(),
                    sidechain_files: Vec::new(),
                    project_root: None,
                    snippet: header.snippet.clone(),
                });
        }

        // NOTE: Compute latest_mod_time for each session after all files are collected
        // This tracks when the session was last active (most recent file modification)
        // Critical for watch mode to identify "most recently updated" vs "most recently created" sessions
        for session in sessions.values_mut() {
            let all_files = vec![session.main_file.as_path()];
            session.latest_mod_time = get_latest_mod_time(&all_files);
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

            if let Ok(header) = extract_gemini_header(path)
                && header.session_id.as_deref() == Some(session_id)
            {
                matching_files.push(path.to_path_buf());
            }
        }

        Ok(matching_files)
    }
}
