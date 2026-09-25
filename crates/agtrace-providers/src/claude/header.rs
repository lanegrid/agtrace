//! Lenient Claude Code file header: agent identity from path + first records (+ sidecars).

use agtrace_types::{AgentId, AgentKind, AgentRef};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::path::{Path, PathBuf};

use super::sidecar::read_subagent_meta;
use crate::Result;
use crate::provider::{FileHeader, read_head_lines};

/// Maximum number of lines read for the header.
const HEADER_MAX_LINES: usize = 64;

/// `<...>/<sessionId>/subagents/agent-<agentId>.jsonl` → (sessionId, agentId)
pub(crate) fn subagent_path_ids(path: &Path) -> Option<(String, String)> {
    let stem = path.file_stem()?.to_str()?;
    let agent_id = stem.strip_prefix("agent-")?;
    let subagents_dir = path.parent()?;
    if subagents_dir.file_name()?.to_str()? != "subagents" {
        return None;
    }
    let session_id = subagents_dir.parent()?.file_name()?.to_str()?;
    if agent_id.is_empty() || session_id.is_empty() {
        return None;
    }
    Some((session_id.to_string(), agent_id.to_string()))
}

#[derive(Default)]
struct HeadScan {
    session_id: Option<String>,
    cwd: Option<String>,
    started_at: Option<DateTime<Utc>>,
    sidechain_agent_id: Option<String>,
    team_name: Option<String>,
    agent_name: Option<String>,
    first_type: Option<String>,
    /// `agent-setting.agentSetting` (teammate agent type; first line of teammate files).
    agent_setting: Option<String>,
    /// Latest `agent-name.agentName` within the header window.
    display_name: Option<String>,
    /// Latest `ai-title.aiTitle` within the header window.
    ai_title: Option<String>,
    lines: usize,
}

fn str_field(v: &Value, key: &str) -> Option<String> {
    v.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// Record kinds that carry the common envelope (`sessionId`, `teamName`, `agentName`, ...).
fn is_transcript_record(kind: &str) -> bool {
    matches!(kind, "user" | "assistant" | "attachment" | "system")
}

fn scan_head(lines: &[String]) -> HeadScan {
    let mut scan = HeadScan {
        lines: lines.len(),
        ..Default::default()
    };
    for (i, line) in lines.iter().enumerate() {
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if i == 0 {
            scan.first_type = str_field(&v, "type");
        }
        if scan.session_id.is_none() {
            scan.session_id = str_field(&v, "sessionId");
        }
        if scan.cwd.is_none() {
            scan.cwd = str_field(&v, "cwd");
        }
        if scan.started_at.is_none() {
            scan.started_at = v
                .get("timestamp")
                .and_then(Value::as_str)
                .and_then(|ts| DateTime::parse_from_rfc3339(ts).ok())
                .map(|dt| dt.with_timezone(&Utc));
        }
        if scan.sidechain_agent_id.is_none()
            && v.get("isSidechain").and_then(Value::as_bool) == Some(true)
        {
            scan.sidechain_agent_id = str_field(&v, "agentId");
        }
        let kind = v.get("type").and_then(Value::as_str).unwrap_or("");
        // Teammate identity comes from the transcript-record envelope only: the
        // `agent-name` state record also has an `agentName` field, but it holds the
        // process display name (a teammate inherits its lead's), not the teammate name.
        if is_transcript_record(kind) {
            if scan.team_name.is_none() {
                scan.team_name = str_field(&v, "teamName");
            }
            if scan.agent_name.is_none() {
                scan.agent_name = str_field(&v, "agentName");
            }
        }
        match kind {
            "agent-setting" if scan.agent_setting.is_none() => {
                scan.agent_setting = str_field(&v, "agentSetting");
            }
            "agent-name" => {
                if let Some(name) = str_field(&v, "agentName") {
                    scan.display_name = Some(name);
                }
            }
            "ai-title" => {
                if let Some(title) = str_field(&v, "aiTitle") {
                    scan.ai_title = Some(title);
                }
            }
            _ => {}
        }
        let is_conversation = kind == "user" || kind == "assistant";
        if is_conversation && scan.session_id.is_some() && scan.cwd.is_some() {
            break;
        }
    }
    scan
}

/// Read the header of a Claude Code transcript. `Ok(None)` for empty files.
pub fn read_claude_header(path: &Path) -> Result<Option<FileHeader>> {
    let lines = read_head_lines(path, HEADER_MAX_LINES)?;
    let scan = scan_head(&lines);
    if scan.lines == 0 {
        return Ok(None);
    }

    let file_stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_string();
    let path_ids = subagent_path_ids(path);

    let session_id = scan
        .session_id
        .clone()
        .or_else(|| path_ids.as_ref().map(|(sid, _)| sid.clone()))
        .unwrap_or_else(|| file_stem.clone());

    // Subagent: by location (<sid>/subagents/agent-<aid>.jsonl) or by content (legacy
    // flat agent files with isSidechain + agentId).
    let subagent_id = path_ids
        .as_ref()
        .map(|(_, aid)| aid.clone())
        .or_else(|| scan.sidechain_agent_id.clone());

    let mut agent = match &subagent_id {
        Some(aid) => {
            let parent = AgentId::claude_session(&session_id);
            let mut r = AgentRef::root(
                AgentId::claude_subagent(&session_id, aid),
                session_id.clone(),
                path.to_path_buf(),
            );
            // Sidecar is optional (may not be written yet); the graph retries it.
            let meta = read_subagent_meta(path).unwrap_or_default();
            r.kind = if scan.first_type.as_deref() == Some("fork-context-ref") || meta.is_fork {
                AgentKind::Fork
            } else {
                AgentKind::Subagent
            };
            r.root = parent.clone();
            r.parent = Some(parent);
            r.native_agent_id = Some(aid.clone());
            r.agent_type = meta.agent_type;
            r.name = meta.name.or(meta.description);
            r.spawn_call_id = meta.tool_use_id;
            r.depth = meta.spawn_depth.unwrap_or(1);
            r
        }
        None => {
            let mut r = AgentRef::root(
                AgentId::claude_session(&session_id),
                session_id.clone(),
                path.to_path_buf(),
            );
            if let (Some(team), Some(name)) = (&scan.team_name, &scan.agent_name) {
                r.kind = AgentKind::Teammate;
                r.native_agent_id = Some(format!("{name}@{team}"));
                r.team = Some(team.clone());
                r.name = Some(name.clone());
                r.agent_type = scan.agent_setting.clone();
            } else {
                r.name = scan.display_name.clone().or_else(|| scan.ai_title.clone());
            }
            r
        }
    };
    agent.cwd = scan.cwd.as_ref().map(PathBuf::from);
    agent.started_at = scan.started_at;

    Ok(Some(FileHeader {
        project_cwd: agent.cwd.clone(),
        agent,
        title: scan.ai_title,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write(dir: &Path, rel: &str, content: &str) -> PathBuf {
        let path = dir.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        path
    }

    const USER: &str = r#"{"type":"user","uuid":"u1","parentUuid":null,"sessionId":"00000000-0000-4000-8000-000000000001","timestamp":"2026-09-20T10:00:00Z","cwd":"/work/demo-project","message":{"role":"user","content":"hi"}}"#;

    #[test]
    fn main_transcript() {
        let dir = tempfile::tempdir().unwrap();
        let path = write(
            dir.path(),
            "proj/00000000-0000-4000-8000-000000000001.jsonl",
            &format!("{{\"type\":\"mode\",\"mode\":\"x\"}}\n{USER}\n"),
        );
        let h = read_claude_header(&path).unwrap().unwrap();
        assert_eq!(
            h.agent.id.as_str(),
            "claude:00000000-0000-4000-8000-000000000001"
        );
        assert_eq!(h.agent.kind, AgentKind::Main);
        assert_eq!(h.agent.parent, None);
        assert_eq!(h.project_cwd, Some(PathBuf::from("/work/demo-project")));
        assert!(h.agent.started_at.is_some());
    }

    #[test]
    fn teammate_transcript() {
        let line = USER.replace(
            "\"cwd\"",
            "\"teamName\":\"session-00000001\",\"agentName\":\"audit-A\",\"cwd\"",
        );
        let dir = tempfile::tempdir().unwrap();
        let path = write(dir.path(), "proj/t.jsonl", &format!("{line}\n"));
        let h = read_claude_header(&path).unwrap().unwrap();
        assert_eq!(h.agent.kind, AgentKind::Teammate);
        assert_eq!(
            h.agent.native_agent_id.as_deref(),
            Some("audit-A@session-00000001")
        );
        assert_eq!(h.agent.team.as_deref(), Some("session-00000001"));
    }

    /// A teammate's `agent-name` record holds the display name inherited from its
    /// lead; the envelope `agentName` (the teammate name) must win even when the
    /// state record comes first.
    #[test]
    fn teammate_name_ignores_inherited_agent_name_record() {
        let line = USER.replace(
            "\"cwd\"",
            "\"teamName\":\"session-00000001\",\"agentName\":\"worker-1\",\"cwd\"",
        );
        let dir = tempfile::tempdir().unwrap();
        let path = write(
            dir.path(),
            "proj/t.jsonl",
            &format!(
                "{{\"type\":\"agent-setting\",\"agentSetting\":\"general-purpose\"}}\n\
                 {{\"type\":\"agent-name\",\"agentName\":\"Lead display name\"}}\n{line}\n"
            ),
        );
        let h = read_claude_header(&path).unwrap().unwrap();
        assert_eq!(h.agent.kind, AgentKind::Teammate);
        assert_eq!(h.agent.name.as_deref(), Some("worker-1"));
        assert_eq!(
            h.agent.native_agent_id.as_deref(),
            Some("worker-1@session-00000001")
        );
    }

    #[test]
    fn subagent_by_location() {
        let dir = tempfile::tempdir().unwrap();
        let path = write(
            dir.path(),
            "proj/00000000-0000-4000-8000-000000000001/subagents/agent-a0000000000000001.jsonl",
            &format!("{USER}\n"),
        );
        let h = read_claude_header(&path).unwrap().unwrap();
        assert_eq!(
            h.agent.id.as_str(),
            "claude:00000000-0000-4000-8000-000000000001/a0000000000000001"
        );
        assert_eq!(h.agent.kind, AgentKind::Subagent);
        assert_eq!(
            h.agent.parent.as_ref().map(|p| p.as_str()),
            Some("claude:00000000-0000-4000-8000-000000000001")
        );
    }

    #[test]
    fn subagent_identity_from_meta_sidecar() {
        let dir = tempfile::tempdir().unwrap();
        let path = write(
            dir.path(),
            "proj/00000000-0000-4000-8000-000000000001/subagents/agent-a0000000000000002.jsonl",
            &format!("{USER}\n"),
        );
        // Without meta: still an agent (meta may be written later).
        let h = read_claude_header(&path).unwrap().unwrap();
        assert_eq!(h.agent.kind, AgentKind::Subagent);
        assert_eq!(h.agent.spawn_call_id, None);
        std::fs::write(
            path.with_extension("meta.json"),
            r#"{"agentType":"fork","description":"d","toolUseId":"toolu_synthetic_9","spawnDepth":2,"isFork":true,"name":"docs-fork"}"#,
        )
        .unwrap();
        let h = read_claude_header(&path).unwrap().unwrap();
        assert_eq!(h.agent.kind, AgentKind::Fork);
        assert_eq!(h.agent.spawn_call_id.as_deref(), Some("toolu_synthetic_9"));
        assert_eq!(h.agent.agent_type.as_deref(), Some("fork"));
        assert_eq!(h.agent.name.as_deref(), Some("docs-fork"));
        assert_eq!(h.agent.depth, 2);
    }

    #[test]
    fn main_name_and_title_from_state_records() {
        let dir = tempfile::tempdir().unwrap();
        let path = write(
            dir.path(),
            "proj/m.jsonl",
            &format!(
                "{{\"type\":\"ai-title\",\"aiTitle\":\"Old\"}}\n{{\"type\":\"ai-title\",\"aiTitle\":\"New\"}}\n{USER}\n"
            ),
        );
        let h = read_claude_header(&path).unwrap().unwrap();
        assert_eq!(h.title.as_deref(), Some("New"));
        assert_eq!(h.agent.name.as_deref(), Some("New"));
    }

    #[test]
    fn empty_file_is_not_an_agent_yet() {
        let dir = tempfile::tempdir().unwrap();
        let path = write(dir.path(), "proj/x.jsonl", "");
        assert!(read_claude_header(&path).unwrap().is_none());
    }
}
