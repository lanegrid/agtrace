use crate::traits::{LogDiscovery, ProbeResult, SessionIndex};
use crate::{Error, Result};
use agtrace_types::SpawnContext;
use chrono::DateTime;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use super::io::{SpawnEvent, extract_codex_header, extract_spawn_events, is_empty_codex_session};

pub struct CodexDiscovery;

impl LogDiscovery for CodexDiscovery {
    fn id(&self) -> &'static str {
        "codex"
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

        let is_jsonl = path.extension().is_some_and(|e| e == "jsonl");
        let filename = path.file_name().and_then(|f| f.to_str()).unwrap_or("");

        if is_jsonl && filename.starts_with("rollout-") && !is_empty_codex_session(path) {
            ProbeResult::match_high()
        } else {
            ProbeResult::NoMatch
        }
    }

    fn resolve_log_root(&self, _project_root: &Path) -> Option<PathBuf> {
        None
    }

    fn scan_sessions(&self, log_root: &Path) -> Result<Vec<SessionIndex>> {
        if !log_root.exists() {
            return Ok(Vec::new());
        }

        // Phase 1: Collect all sessions, separating CLI and subagent sessions
        let mut cli_sessions: Vec<(SessionIndex, PathBuf)> = Vec::new();
        let mut subagent_sessions: Vec<(SessionIndex, String)> = Vec::new(); // (session, timestamp)

        for entry in WalkDir::new(log_root).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();

            if self.probe(path) == ProbeResult::NoMatch {
                continue;
            }

            let header = match extract_codex_header(path) {
                Ok(h) => h,
                Err(_) => continue,
            };

            let session_id = match header.session_id {
                Some(id) => id,
                None => continue,
            };

            let session = SessionIndex {
                session_id: session_id.clone(),
                timestamp: header.timestamp.clone(),
                latest_mod_time: None,
                main_file: path.to_path_buf(),
                sidechain_files: Vec::new(),
                project_root: header.cwd.clone().map(PathBuf::from),
                repository_hash: None, // Computed at index time from project_root
                snippet: header.snippet.clone(),
                parent_session_id: None,
                spawned_by: None,
            };

            if header.subagent_type.is_some() {
                // Subagent session - store with timestamp for correlation
                let ts = header.timestamp.clone().unwrap_or_default();
                subagent_sessions.push((session, ts));
            } else {
                // CLI session - store with path for spawn event extraction
                cli_sessions.push((session, path.to_path_buf()));
            }
        }

        // Phase 2: Build spawn event map from CLI sessions
        // Map: (parent_session_id, spawn_timestamp) -> SpawnContext
        let mut spawn_map: HashMap<String, Vec<SpawnEvent>> = HashMap::new();

        for (session, path) in &cli_sessions {
            if let Ok(spawn_events) = extract_spawn_events(path)
                && !spawn_events.is_empty()
            {
                spawn_map.insert(session.session_id.clone(), spawn_events);
            }
        }

        // Phase 3: Correlate subagent sessions to parent spawn events
        for (session, subagent_ts) in &mut subagent_sessions {
            if let Some((parent_id, spawn_ctx)) =
                find_matching_spawn(&spawn_map, subagent_ts, session.project_root.as_ref())
            {
                session.parent_session_id = Some(parent_id);
                session.spawned_by = Some(spawn_ctx);
            }
        }

        // Combine all sessions
        let mut all_sessions: HashMap<String, SessionIndex> = HashMap::new();

        for (session, _) in cli_sessions {
            all_sessions.insert(session.session_id.clone(), session);
        }
        for (session, _) in subagent_sessions {
            all_sessions.insert(session.session_id.clone(), session);
        }

        // Compute latest_mod_time for each session
        for session in all_sessions.values_mut() {
            let all_files = vec![session.main_file.as_path()];
            session.latest_mod_time = crate::get_latest_mod_time_rfc3339(&all_files);
        }

        Ok(all_sessions.into_values().collect())
    }

    fn extract_session_id(&self, path: &Path) -> Result<String> {
        let header = extract_codex_header(path)?;
        header
            .session_id
            .ok_or_else(|| Error::Parse(format!("No session_id in file: {}", path.display())))
    }

    fn extract_project_hash(&self, path: &Path) -> Result<Option<agtrace_types::ProjectHash>> {
        let header = extract_codex_header(path)?;
        Ok(header
            .cwd
            .map(|cwd| agtrace_core::project_hash_from_root(&cwd)))
    }

    fn find_session_files(&self, log_root: &Path, session_id: &str) -> Result<Vec<PathBuf>> {
        let mut matching_files = Vec::new();

        for entry in WalkDir::new(log_root).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();

            if self.probe(path) == ProbeResult::NoMatch {
                continue;
            }

            if let Ok(header) = extract_codex_header(path)
                && header.session_id.as_deref() == Some(session_id)
            {
                matching_files.push(path.to_path_buf());
            }
        }

        Ok(matching_files)
    }

    fn is_sidechain_file(&self, path: &Path) -> Result<bool> {
        let header = extract_codex_header(path)?;
        Ok(header.subagent_type.is_some())
    }
}

/// Find a matching spawn event for a subagent based on timestamp correlation.
/// Returns (parent_session_id, SpawnContext) if found within 100ms window.
fn find_matching_spawn(
    spawn_map: &HashMap<String, Vec<SpawnEvent>>,
    subagent_ts: &str,
    subagent_project: Option<&PathBuf>,
) -> Option<(String, SpawnContext)> {
    let subagent_dt = DateTime::parse_from_rfc3339(subagent_ts).ok()?;

    // Maximum time difference for correlation (100ms)
    const MAX_DIFF_MS: i64 = 100;

    for (parent_id, spawn_events) in spawn_map {
        for spawn in spawn_events {
            let spawn_dt = match DateTime::parse_from_rfc3339(&spawn.timestamp) {
                Ok(dt) => dt,
                Err(_) => continue,
            };

            // Calculate time difference in milliseconds
            let diff = (subagent_dt.timestamp_millis() - spawn_dt.timestamp_millis()).abs();

            if diff <= MAX_DIFF_MS {
                // Found a match within the time window
                return Some((parent_id.clone(), spawn.spawn_context.clone()));
            }
        }
    }

    // No match found - subagent might be from a different parent or standalone
    let _ = subagent_project; // Future: could also match by project_root
    None
}

/// File name of the Codex thread title index inside the Codex home directory.
pub const SESSION_INDEX_FILE: &str = "session_index.jsonl";

/// Read `session_index.jsonl` (`{id, thread_name, updated_at}` per line) into
/// `thread id -> title`. Later lines win (renames append a new line). Lenient: missing
/// files and bad lines are skipped.
pub fn read_session_index(path: &Path) -> HashMap<String, String> {
    let mut titles = HashMap::new();
    let Ok(text) = std::fs::read_to_string(path) else {
        return titles;
    };
    for line in text.lines() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let id = v.get("id").and_then(|x| x.as_str());
        let name = v.get("thread_name").and_then(|x| x.as_str());
        if let (Some(id), Some(name)) = (id, name)
            && !name.is_empty()
        {
            titles.insert(id.to_string(), name.to_string());
        }
    }
    titles
}

#[cfg(test)]
mod session_index_tests {
    use super::*;

    #[test]
    fn latest_title_wins_and_bad_lines_are_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(SESSION_INDEX_FILE);
        std::fs::write(
            &path,
            "{\"id\":\"t1\",\"thread_name\":\"first\",\"updated_at\":\"2026-09-20T10:00:00Z\"}\n\
             not json\n\
             {\"id\":\"t2\",\"thread_name\":\"other\"}\n\
             {\"id\":\"t1\",\"thread_name\":\"renamed\"}\n",
        )
        .unwrap();
        let titles = read_session_index(&path);
        assert_eq!(titles.get("t1").map(String::as_str), Some("renamed"));
        assert_eq!(titles.get("t2").map(String::as_str), Some("other"));
        assert!(read_session_index(&dir.path().join("missing.jsonl")).is_empty());
    }
}
