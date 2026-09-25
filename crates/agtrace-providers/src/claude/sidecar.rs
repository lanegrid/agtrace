//! Claude Code sidecar files next to agent transcripts.
//!
//! `<sid>/subagents/agent-<aid>.meta.json`:
//! `{agentType, description, toolUseId, spawnDepth, model, [requestShape, requestNonInteractive],
//!   [isFork, name], [stoppedByUser]}` — written at spawn time.
//!
//! Claude home side state (outside `projects/`):
//! - `teams/<team>/config.json`: `{name, leadSessionId, members[{name, agentType, model, isActive}]}`
//! - `sessions/<pid>.json`: process registry `{pid, sessionId, cwd, status, name, updatedAt}`.
//!   The sibling `<pid>.<hash>.key` files are secrets and are never read.

use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Parsed `agent-<aid>.meta.json` (all fields optional; unknown fields ignored).
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeSubagentMeta {
    /// `general-purpose`, `Explore`, `fork`, ...
    #[serde(default)]
    pub agent_type: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    /// Provider call id of the spawning `Agent` tool_use in the parent transcript.
    #[serde(default)]
    pub tool_use_id: Option<String>,
    #[serde(default)]
    pub spawn_depth: Option<u32>,
    /// Requested model alias or id (`opus`, `claude-opus-5-5[1m]`).
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub request_shape: Option<String>,
    #[serde(default)]
    pub is_fork: bool,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub stopped_by_user: bool,
}

/// `<dir>/agent-<aid>.jsonl` -> `<dir>/agent-<aid>.meta.json`
pub fn subagent_meta_path(transcript: &Path) -> PathBuf {
    transcript.with_extension("meta.json")
}

/// Read the meta sidecar of a subagent transcript. `None` if missing or unreadable
/// (it may not be written yet; callers retry later).
pub fn read_subagent_meta(transcript: &Path) -> Option<ClaudeSubagentMeta> {
    let text = std::fs::read_to_string(subagent_meta_path(transcript)).ok()?;
    serde_json::from_str(&text).ok()
}

/// One `members[]` entry of a team config.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeTeamMemberConfig {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub agent_type: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub is_active: Option<bool>,
}

/// `~/.claude/teams/<team>/config.json` (lenient; unknown fields ignored).
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeTeamConfig {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub lead_session_id: Option<String>,
    #[serde(default)]
    pub members: Vec<ClaudeTeamMemberConfig>,
}

/// `<claude home>/teams`
pub fn teams_dir(claude_home: &Path) -> PathBuf {
    claude_home.join("teams")
}

/// Read a team config. The team name falls back to the directory name.
pub fn read_team_config(config_path: &Path) -> Option<ClaudeTeamConfig> {
    let text = std::fs::read_to_string(config_path).ok()?;
    let mut config: ClaudeTeamConfig = serde_json::from_str(&text).ok()?;
    if config.name.is_empty() {
        config.name = config_path
            .parent()?
            .file_name()?
            .to_string_lossy()
            .into_owned();
    }
    Some(config)
}

/// `teams/*/config.json` paths (no content read).
pub fn team_config_paths(claude_home: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(teams_dir(claude_home)) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path().join("config.json"))
        .filter(|p| p.is_file())
        .collect();
    paths.sort();
    paths
}

/// Process registry status (`sessions/<pid>.json` `status`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaudeProcessStatus {
    Busy,
    Idle,
    #[serde(other)]
    Other,
}

/// One `~/.claude/sessions/<pid>.json` entry (lenient; unknown fields ignored).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeProcessEntry {
    pub pid: u32,
    pub session_id: String,
    #[serde(default)]
    pub cwd: Option<PathBuf>,
    #[serde(default)]
    pub status: Option<ClaudeProcessStatus>,
    #[serde(default)]
    pub name: Option<String>,
    /// How `name` was chosen: `derived` (from the working directory, e.g.
    /// `yohaku-studio-c8`), `auto` (title / job id), or set by the user.
    #[serde(default)]
    pub name_source: Option<String>,
    /// `interactive` or `bg` (background daemon / job session).
    #[serde(default)]
    pub kind: Option<String>,
    /// Epoch milliseconds.
    #[serde(default)]
    pub updated_at: Option<i64>,
    /// Epoch milliseconds.
    #[serde(default)]
    pub started_at: Option<i64>,
}

/// `<claude home>/sessions`
pub fn sessions_registry_dir(claude_home: &Path) -> PathBuf {
    claude_home.join("sessions")
}

/// Registry entry paths: `sessions/<pid>.json` only. Anything else (in particular the
/// `<pid>.<hash>.key` secrets) is skipped by name, never opened.
pub fn session_registry_paths(claude_home: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(sessions_registry_dir(claude_home)) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| is_registry_entry_path(p))
        .collect();
    paths.sort();
    paths
}

/// `<digits>.json` (e.g. `4242.json`); rejects `4242.<hash>.key` and other files.
pub fn is_registry_entry_path(path: &Path) -> bool {
    path.extension().is_some_and(|e| e == "json")
        && path
            .file_stem()
            .and_then(|s| s.to_str())
            .is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
}

/// Read one registry entry (`None` if unreadable / not a registry entry).
pub fn read_process_entry(path: &Path) -> Option<ClaudeProcessEntry> {
    if !is_registry_entry_path(path) {
        return None;
    }
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_meta_variants() {
        let dir = tempfile::tempdir().unwrap();
        let transcript = dir.path().join("agent-a0000000000000002.jsonl");
        assert_eq!(read_subagent_meta(&transcript), None);
        std::fs::write(
            subagent_meta_path(&transcript),
            r#"{"agentType":"fork","description":"d","toolUseId":"toolu_synthetic_1","spawnDepth":1,"model":"opus","isFork":true,"name":"docs-fork","extra":{"x":1}}"#,
        )
        .unwrap();
        let meta = read_subagent_meta(&transcript).unwrap();
        assert_eq!(meta.agent_type.as_deref(), Some("fork"));
        assert_eq!(meta.tool_use_id.as_deref(), Some("toolu_synthetic_1"));
        assert_eq!(meta.spawn_depth, Some(1));
        assert!(meta.is_fork);
        assert!(!meta.stopped_by_user);
        assert_eq!(meta.name.as_deref(), Some("docs-fork"));
    }

    #[test]
    fn registry_paths_never_include_key_files() {
        let dir = tempfile::tempdir().unwrap();
        let sessions = sessions_registry_dir(dir.path());
        std::fs::create_dir_all(&sessions).unwrap();
        std::fs::write(
            sessions.join("4242.json"),
            r#"{"pid":4242,"sessionId":"s1","status":"busy","updatedAt":1789898460000,"x":1}"#,
        )
        .unwrap();
        std::fs::write(sessions.join("4242.deadbeef.key"), "secret").unwrap();
        std::fs::write(sessions.join("notes.json"), "{}").unwrap();
        let paths = session_registry_paths(dir.path());
        assert_eq!(paths, vec![sessions.join("4242.json")]);
        assert!(read_process_entry(&sessions.join("4242.deadbeef.key")).is_none());
        let entry = read_process_entry(&paths[0]).unwrap();
        assert_eq!(entry.session_id, "s1");
        assert_eq!(entry.status, Some(ClaudeProcessStatus::Busy));
        assert_eq!(entry.updated_at, Some(1789898460000));
    }

    #[test]
    fn team_config_is_lenient_and_named_after_dir() {
        let dir = tempfile::tempdir().unwrap();
        let team = teams_dir(dir.path()).join("t1");
        std::fs::create_dir_all(&team).unwrap();
        std::fs::write(
            team.join("config.json"),
            r#"{"leadSessionId":"s1","members":[{"name":"team-lead"},{"name":"audit-A","agentType":"general-purpose","model":"claude-opus-5-5[1m]","isActive":false,"extra":2}]}"#,
        )
        .unwrap();
        assert_eq!(
            team_config_paths(dir.path()),
            vec![team.join("config.json")]
        );
        let config = read_team_config(&team.join("config.json")).unwrap();
        assert_eq!(config.name, "t1");
        assert_eq!(config.lead_session_id.as_deref(), Some("s1"));
        assert_eq!(config.members[1].is_active, Some(false));
        assert_eq!(config.members[0].is_active, None);
    }
}
