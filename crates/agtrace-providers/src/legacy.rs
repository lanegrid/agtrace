use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::traits::ProviderAdapter;

#[derive(Debug, Clone)]
pub struct LogFileMetadata {
    pub path: String,
    pub role: String,
    pub file_size: Option<i64>,
    pub mod_time: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SessionMetadata {
    pub session_id: String,
    pub project_hash: String,
    pub project_root: Option<String>,
    pub provider: String,
    pub start_ts: Option<String>,
    pub end_ts: Option<String>,
    pub snippet: Option<String>,
    pub log_files: Vec<LogFileMetadata>,
}

pub struct ScanContext {
    pub project_hash: String,
    pub project_root: Option<String>,
    pub provider_filter: Option<String>,
}

impl ProviderAdapter {
    /// Legacy support: Scan sessions and convert to full SessionMetadata
    ///
    /// This bridges the gap between the new efficient SessionIndex and the old UI-heavy SessionMetadata.
    /// The new discovery layer provides lightweight SessionIndex, and this method enriches it with
    /// file metadata for backward compatibility with existing UI code.
    pub fn scan_legacy(
        &self,
        log_root: &Path,
        context: &ScanContext,
    ) -> Result<Vec<SessionMetadata>> {
        let sessions = self.discovery.scan_sessions(log_root)?;

        let mut metadata_list = Vec::new();

        for session in sessions {
            if let Some(expected_root) = &context.project_root {
                if let Some(session_root) = &session.project_root {
                    let session_normalized = session_root.trim_end_matches('/');
                    let expected_normalized = expected_root.trim_end_matches('/');
                    if session_normalized != expected_normalized {
                        continue;
                    }
                } else if self.id() != "gemini" {
                    continue;
                }
            }

            let mut log_files = Vec::new();

            let to_log_file = |path: &PathBuf, role: &str| -> LogFileMetadata {
                let meta = std::fs::metadata(path).ok();
                LogFileMetadata {
                    path: path.display().to_string(),
                    role: role.to_string(),
                    file_size: meta.as_ref().map(|m| m.len() as i64),
                    mod_time: meta
                        .and_then(|m| m.modified().ok())
                        .map(|t| format!("{:?}", t)),
                }
            };

            log_files.push(to_log_file(&session.main_file, "main"));

            for side_file in &session.sidechain_files {
                log_files.push(to_log_file(side_file, "sidechain"));
            }

            let project_hash = if let Some(ref root) = session.project_root {
                agtrace_types::project_hash_from_root(root)
            } else if self.id() == "gemini" {
                // For Gemini, extract project_hash directly from the file
                use crate::gemini::io::extract_project_hash_from_gemini_file;
                extract_project_hash_from_gemini_file(&session.main_file)
                    .unwrap_or_else(|| context.project_hash.clone())
            } else {
                context.project_hash.clone()
            };

            metadata_list.push(SessionMetadata {
                session_id: session.session_id,
                project_hash,
                project_root: session.project_root,
                provider: self.id().to_string(),
                start_ts: session.timestamp,
                end_ts: None,
                snippet: session.snippet,
                log_files,
            });
        }

        Ok(metadata_list)
    }
}
