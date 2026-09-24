//! Claude Code sidecar files next to agent transcripts.
//!
//! `<sid>/subagents/agent-<aid>.meta.json`:
//! `{agentType, description, toolUseId, spawnDepth, model, [requestShape, requestNonInteractive],
//!   [isFork, name], [stoppedByUser]}` — written at spawn time.

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
}
