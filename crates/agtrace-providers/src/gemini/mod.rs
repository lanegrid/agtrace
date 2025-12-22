pub mod io;
pub mod mapper;
pub mod models;
pub mod parser;
pub mod schema;
pub mod tool_mapping;
pub mod tools;

use crate::traits::{ProbeResult, SessionIndex};
use agtrace_types::project_hash_from_root;
use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub use self::io::{
    extract_gemini_header, extract_project_hash_from_gemini_file, normalize_gemini_file,
};
pub use self::mapper::GeminiToolMapper;
pub use self::parser::GeminiParser;

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
                    project_root: None,          // Gemini uses project_hash instead
                    snippet: header.snippet.clone(),
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

// --- Backward-compatible provider (Facade pattern) ---
