//! Claude Code file discovery.
//!
//! - Project dir = `<projects root>/<cwd with every non-alphanumeric char replaced by '-'>`
//!   plus the directories listed in its `.session-aliases` file.
//! - Agent files: `<project>/<sessionId>.jsonl` (main / teammate transcripts) and
//!   `<project>/<sessionId>/subagents/agent-<agentId>.jsonl`. `tool-results/` is not.
//! - [`ClaudeDiscovery`] is the legacy index scanner (`LogDiscovery`), kept until the
//!   index is rebuilt on `Provider::discover`.

use crate::traits::{LogDiscovery, ProbeResult, SessionIndex};
use crate::{Error, Result};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Claude's project directory name for a working directory
/// (`/work/demo-project` -> `-work-demo-project`).
pub fn encode_project_dir(cwd: &Path) -> String {
    cwd.to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

/// Location rule for agent files (no content read): `*.jsonl` directly in a project
/// dir, or `agent-*.jsonl` in a `subagents/` dir. Anything under `tool-results/` is not.
pub fn is_agent_file_path(path: &Path) -> bool {
    if path.extension().is_none_or(|e| e != "jsonl") {
        return false;
    }
    let parent = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("");
    match parent {
        "tool-results" => false,
        "subagents" => path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with("agent-")),
        _ => true,
    }
}

/// Project directories under `projects_root` that may hold agents of `project_root`:
/// dirs whose name starts with the encoded project path (the project itself and
/// directories below it, e.g. worktrees), plus their `.session-aliases` entries.
/// Candidates are over-approximated; callers filter by header cwd.
pub fn project_dirs(projects_root: &Path, project_root: &Path) -> Vec<PathBuf> {
    let prefix = encode_project_dir(project_root);
    let mut dirs = Vec::new();
    let mut seen = HashSet::new();
    let Ok(entries) = std::fs::read_dir(projects_root) else {
        return dirs;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let matches = entry
            .file_name()
            .to_str()
            .is_some_and(|n| n.starts_with(&prefix));
        if !matches || !path.is_dir() {
            continue;
        }
        for alias in read_session_aliases(&path) {
            if alias.is_dir() && seen.insert(alias.clone()) {
                dirs.push(alias);
            }
        }
        if seen.insert(path.clone()) {
            dirs.push(path);
        }
    }
    dirs.sort();
    dirs
}

/// `.session-aliases`: one absolute path of another project dir per line.
fn read_session_aliases(project_dir: &Path) -> Vec<PathBuf> {
    std::fs::read_to_string(project_dir.join(".session-aliases"))
        .map(|text| {
            text.lines()
                .map(str::trim)
                .filter(|l| !l.is_empty() && Path::new(l).is_absolute())
                .map(PathBuf::from)
                .collect()
        })
        .unwrap_or_default()
}

/// Agent files of one project dir (main/teammate transcripts + subagent transcripts).
pub fn agent_files_in(project_dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(entries) = std::fs::read_dir(project_dir) else {
        return files;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if is_agent_file_path(&path) {
                files.push(path);
            }
        } else if path.is_dir()
            && let Ok(subs) = std::fs::read_dir(path.join("subagents"))
        {
            files.extend(
                subs.flatten()
                    .map(|e| e.path())
                    .filter(|p| p.is_file() && is_agent_file_path(p)),
            );
        }
    }
    files.sort();
    files
}

/// Parse a Claude Code JSONL file and normalize to AgentEvent (lenient per line).
pub fn normalize_claude_file(path: &Path) -> Result<Vec<agtrace_types::AgentEvent>> {
    let (_, events, _) = crate::provider::decode_file(
        &super::ClaudeProvider,
        path,
        crate::provider::DecodeOptions::default(),
    )?;
    Ok(events)
}

fn head_records(path: &Path, max: usize) -> Result<Vec<Value>> {
    let file = std::fs::File::open(path)?;
    Ok(BufReader::new(file)
        .lines()
        .take(max)
        .map_while(|l| l.ok())
        .filter_map(|l| serde_json::from_str::<Value>(&l).ok())
        .collect())
}

fn vstr(v: &Value, key: &str) -> Option<String> {
    v.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// Extract cwd from a Claude session file by reading the first few lines
pub fn extract_cwd_from_claude_file(path: &Path) -> Option<String> {
    head_records(path, 10).ok()?.iter().find_map(|v| {
        matches!(
            v.get("type").and_then(Value::as_str),
            Some("user" | "assistant")
        )
        .then(|| vstr(v, "cwd"))
        .flatten()
    })
}

#[derive(Debug)]
pub struct ClaudeHeader {
    pub session_id: Option<String>,
    pub cwd: Option<String>,
    pub timestamp: Option<String>,
    pub snippet: Option<String>,
    pub is_sidechain: bool,
    pub subagent_id: Option<String>,
}

/// First text of a user message (string content or first text block).
fn user_text(v: &Value) -> Option<&str> {
    let content = v.get("message")?.get("content")?;
    match content {
        Value::String(s) => Some(s),
        Value::Array(items) => items.iter().find_map(|i| {
            (i.get("type").and_then(Value::as_str) == Some("text"))
                .then(|| i.get("text").and_then(Value::as_str))
                .flatten()
        }),
        _ => None,
    }
}

/// Extract header information from Claude file (legacy index scanning).
pub fn extract_claude_header(path: &Path) -> Result<ClaudeHeader> {
    let mut session_id = None;
    let mut cwd = None;
    let mut timestamp = None;
    let mut snippet = None;
    let mut is_sidechain = false;
    let mut subagent_id = None;
    let mut meta_message_ids = HashSet::new();

    for v in head_records(path, 200)? {
        let kind = v.get("type").and_then(Value::as_str).unwrap_or("");
        match kind {
            "file-history-snapshot" => meta_message_ids.clear(),
            "user" | "assistant" => {
                if session_id.is_none() {
                    session_id = vstr(&v, "sessionId");
                }
                if cwd.is_none() {
                    cwd = vstr(&v, "cwd");
                }
                if timestamp.is_none() {
                    timestamp = vstr(&v, "timestamp");
                }
            }
            _ => {}
        }
        if kind == "user" {
            let uuid = vstr(&v, "uuid").unwrap_or_default();
            let sidechain = v.get("isSidechain").and_then(Value::as_bool) == Some(true);
            let is_meta = v.get("isMeta").and_then(Value::as_bool) == Some(true);
            if is_meta {
                meta_message_ids.insert(uuid.clone());
            }
            // Descendants of meta messages are meta-related too.
            let parent_is_meta =
                vstr(&v, "parentUuid").is_some_and(|p| meta_message_ids.contains(&p));
            if parent_is_meta {
                meta_message_ids.insert(uuid);
            }
            if snippet.is_none() && !sidechain && !is_meta && !parent_is_meta {
                snippet = user_text(&v).map(|t| agtrace_types::truncate(t, 200));
            }
            if subagent_id.is_none() {
                subagent_id = vstr(&v, "agentId");
            }
            if subagent_id.is_none()
                && let Some(Value::Array(items)) = v.get("message").and_then(|m| m.get("content"))
            {
                subagent_id = items.iter().find_map(|i| vstr(i, "agentId"));
            }
            is_sidechain = sidechain;
        }
        if session_id.is_some() && cwd.is_some() && timestamp.is_some() && snippet.is_some() {
            break;
        }
    }

    Ok(ClaudeHeader {
        session_id,
        cwd,
        timestamp,
        snippet,
        is_sidechain,
        subagent_id,
    })
}

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

        if let Ok(metadata) = std::fs::metadata(path)
            && metadata.len() == 0
        {
            return ProbeResult::NoMatch;
        }

        ProbeResult::match_high()
    }

    fn resolve_log_root(&self, _project_root: &Path) -> Option<PathBuf> {
        None
    }

    fn scan_sessions(&self, log_root: &Path) -> Result<Vec<SessionIndex>> {
        let mut sessions: HashMap<String, SessionIndex> = HashMap::new();

        // max_depth(4) to reach:
        // - depth 0: log_root (e.g., ~/.claude/projects/-proj-hash/)
        // - depth 1: session files (e.g., {session_id}.jsonl)
        // - depth 2: session directories (e.g., {session_id}/)
        // - depth 3: subagents directory (e.g., {session_id}/subagents/)
        // - depth 4: sidechain files (e.g., {session_id}/subagents/agent-xxx.jsonl)
        for entry in WalkDir::new(log_root)
            .max_depth(4)
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
                    latest_mod_time: None, // Will be computed after all files are collected
                    main_file: path.to_path_buf(),
                    sidechain_files: Vec::new(),
                    project_root: header.cwd.clone().map(PathBuf::from),
                    repository_hash: None, // Computed at index time from project_root
                    snippet: header.snippet.clone(),
                    parent_session_id: None, // Claude uses in-file sidechain linking
                    spawned_by: None,
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
                    session.project_root = header.cwd.clone().map(PathBuf::from);
                }
                if session.snippet.is_none() {
                    session.snippet = header.snippet.clone();
                }
            }
        }

        // NOTE: Compute latest_mod_time for each session after all files are collected
        // This tracks when the session was last active (most recent file modification)
        // Critical for watch mode to identify "most recently updated" vs "most recently created" sessions
        for session in sessions.values_mut() {
            let mut all_files = vec![session.main_file.as_path()];
            all_files.extend(session.sidechain_files.iter().map(|p| p.as_path()));
            session.latest_mod_time = crate::get_latest_mod_time_rfc3339(&all_files);
        }

        Ok(sessions.into_values().collect())
    }

    fn extract_session_id(&self, path: &Path) -> Result<String> {
        let header = extract_claude_header(path)?;
        header
            .session_id
            .ok_or_else(|| Error::Parse(format!("No session_id in file: {}", path.display())))
    }

    fn extract_project_hash(&self, path: &Path) -> Result<Option<agtrace_types::ProjectHash>> {
        let header = extract_claude_header(path)?;
        Ok(header
            .cwd
            .map(|cwd| agtrace_core::project_hash_from_root(&cwd)))
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

            if let Ok(header) = extract_claude_header(path)
                && header.session_id.as_deref() == Some(session_id)
            {
                matching_files.push(path.to_path_buf());
            }
        }

        Ok(matching_files)
    }

    fn is_sidechain_file(&self, path: &Path) -> Result<bool> {
        let header = extract_claude_header(path)?;
        Ok(header.is_sidechain)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_project_dirs_like_claude() {
        assert_eq!(
            encode_project_dir(Path::new("/work/demo-project")),
            "-work-demo-project"
        );
        assert_eq!(
            encode_project_dir(Path::new("/w/a.b_c/.wt/x")),
            "-w-a-b-c--wt-x"
        );
    }

    #[test]
    fn agent_file_location_rule() {
        assert!(is_agent_file_path(Path::new("/p/-proj/s.jsonl")));
        assert!(is_agent_file_path(Path::new(
            "/p/-proj/s/subagents/agent-a0000000000000001.jsonl"
        )));
        assert!(!is_agent_file_path(Path::new(
            "/p/-proj/s/subagents/other.jsonl"
        )));
        assert!(!is_agent_file_path(Path::new(
            "/p/-proj/s/tool-results/x.jsonl"
        )));
        assert!(!is_agent_file_path(Path::new(
            "/p/-proj/s/subagents/agent-a0000000000000001.meta.json"
        )));
    }

    #[test]
    fn project_dirs_include_subprojects_and_aliases() {
        let root = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        for d in [
            "-work-demo-project",
            "-work-demo-project--wt-a",
            "-work-else",
        ] {
            std::fs::create_dir_all(root.path().join(d)).unwrap();
        }
        std::fs::write(
            root.path().join("-work-demo-project/.session-aliases"),
            format!("{}\n", other.path().display()),
        )
        .unwrap();
        let dirs = project_dirs(root.path(), Path::new("/work/demo-project"));
        let names: Vec<_> = dirs
            .iter()
            .map(|d| d.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(names.contains(&"-work-demo-project".to_string()));
        assert!(names.contains(&"-work-demo-project--wt-a".to_string()));
        assert!(!names.contains(&"-work-else".to_string()));
        assert!(dirs.contains(&other.path().to_path_buf()));
    }
}
