//! Claude Code file discovery.
//!
//! - Project dir = `<projects root>/<cwd with every non-alphanumeric char replaced by '-'>`
//!   plus the directories listed in its `.session-aliases` file.
//! - Agent files: `<project>/<sessionId>.jsonl` (main / teammate transcripts) and
//!   `<project>/<sessionId>/subagents/agent-<agentId>.jsonl`. `tool-results/` is not.
//! - [`read_snippet`] extracts the first user prompt for the session index.

use crate::Result;
use serde_json::Value;
use std::collections::HashSet;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

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

/// First user prompt of a transcript (head read, ≤ 200 records), truncated to 200 chars.
///
/// Skips sidechain records, `isMeta` records and their descendants.
pub fn read_snippet(path: &Path) -> Option<String> {
    let mut meta_message_ids = HashSet::new();
    for v in head_records(path, 200).ok()? {
        if v.get("type").and_then(Value::as_str) != Some("user") {
            continue;
        }
        let uuid = vstr(&v, "uuid").unwrap_or_default();
        let sidechain = v.get("isSidechain").and_then(Value::as_bool) == Some(true);
        let is_meta = v.get("isMeta").and_then(Value::as_bool) == Some(true);
        if is_meta {
            meta_message_ids.insert(uuid.clone());
        }
        // Descendants of meta messages are meta-related too.
        let parent_is_meta = vstr(&v, "parentUuid").is_some_and(|p| meta_message_ids.contains(&p));
        if parent_is_meta {
            meta_message_ids.insert(uuid);
        }
        if sidechain || is_meta || parent_is_meta {
            continue;
        }
        if let Some(text) = user_text(&v) {
            return Some(agtrace_types::truncate(text, 200));
        }
    }
    None
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
