//! Lenient Codex rollout header: agent identity from `session_meta`.

use agtrace_types::{AgentId, AgentKind, AgentRef};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::path::{Path, PathBuf};

use crate::Result;
use crate::provider::{FileHeader, read_head_lines};

/// `session_meta` is line 0 in current rollouts; older ones may be preceded by a few records.
const HEADER_MAX_LINES: usize = 20;

fn str_at<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key).and_then(Value::as_str).filter(|s| !s.is_empty())
}

/// Build the header from a `session_meta` payload.
pub(crate) fn header_from_session_meta(
    payload: &Value,
    record_timestamp: Option<&str>,
    path: &Path,
) -> Option<FileHeader> {
    let thread_id = str_at(payload, "id")?;
    let id = AgentId::codex_thread(thread_id);
    let root_session = str_at(payload, "session_id").unwrap_or(thread_id);

    let source = payload.get("source");
    let subagent = source.and_then(|s| s.get("subagent"));
    let thread_spawn = subagent.and_then(|s| s.get("thread_spawn"));

    let parent_thread_id = str_at(payload, "parent_thread_id")
        .or_else(|| thread_spawn.and_then(|t| str_at(t, "parent_thread_id")));
    let agent_path = str_at(payload, "agent_path")
        .or_else(|| thread_spawn.and_then(|t| str_at(t, "agent_path")));
    let nickname = str_at(payload, "agent_nickname")
        .or_else(|| thread_spawn.and_then(|t| str_at(t, "agent_nickname")));
    let agent_role = thread_spawn
        .and_then(|t| str_at(t, "agent_role"))
        .or_else(|| subagent.and_then(Value::as_str));
    let depth = thread_spawn
        .and_then(|t| t.get("depth"))
        .and_then(Value::as_u64)
        .unwrap_or(0) as u32;
    let forked = str_at(payload, "forked_from_id").is_some();
    let is_child = subagent.is_some() || parent_thread_id.is_some() || forked;

    let mut agent = AgentRef::root(id, thread_id, path.to_path_buf());
    if is_child {
        agent.kind = if forked {
            AgentKind::Fork
        } else {
            AgentKind::CodexThread
        };
        agent.root = AgentId::codex_thread(root_session);
        agent.parent = parent_thread_id.map(AgentId::codex_thread);
        agent.depth = depth.max(1);
    }
    // multi_agent v2 paths: children carry `agent_path`; a root is implicitly `/root`.
    agent.path = agent_path
        .map(str::to_string)
        .or_else(|| (!is_child).then(|| super::collab::ROOT_PATH.to_string()));
    agent.name = agent_path
        .and_then(|p| p.rsplit('/').next())
        .filter(|leaf| !leaf.is_empty())
        .or(nickname)
        .map(str::to_string);
    agent.agent_type = agent_role.map(str::to_string);
    agent.cwd = str_at(payload, "cwd").map(PathBuf::from);
    agent.started_at = str_at(payload, "timestamp")
        .or(record_timestamp)
        .and_then(|ts| DateTime::parse_from_rfc3339(ts).ok())
        .map(|dt| dt.with_timezone(&Utc));

    Some(FileHeader {
        project_cwd: agent.cwd.clone(),
        agent,
        title: None,
    })
}

/// Read the header of a Codex rollout. `Ok(None)` if no `session_meta` is present (yet).
pub fn read_codex_header(path: &Path) -> Result<Option<FileHeader>> {
    for line in read_head_lines(path, HEADER_MAX_LINES)? {
        let Ok(v) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if v.get("type").and_then(Value::as_str) != Some("session_meta") {
            continue;
        }
        let Some(payload) = v.get("payload") else {
            continue;
        };
        return Ok(header_from_session_meta(
            payload,
            v.get("timestamp").and_then(Value::as_str),
            path,
        ));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(payload: Value) -> FileHeader {
        header_from_session_meta(&payload, None, Path::new("/x/rollout.jsonl")).unwrap()
    }

    #[test]
    fn root_thread() {
        let h = header(serde_json::json!({
            "id": "01900000-0000-7000-8000-000000000001",
            "session_id": "01900000-0000-7000-8000-000000000001",
            "cwd": "/work/demo-project",
            "timestamp": "2026-09-20T10:00:00Z",
            "source": "cli"
        }));
        assert_eq!(h.agent.kind, AgentKind::Main);
        assert_eq!(h.agent.id, h.agent.root);
        assert_eq!(h.agent.parent, None);
        assert_eq!(h.agent.path.as_deref(), Some("/root"));
        assert_eq!(h.agent.name, None);
        assert_eq!(h.project_cwd, Some(PathBuf::from("/work/demo-project")));
    }

    #[test]
    fn thread_spawn_child() {
        let h = header(serde_json::json!({
            "id": "01900000-0000-7000-8000-000000000002",
            "session_id": "01900000-0000-7000-8000-000000000001",
            "source": {"subagent": {"thread_spawn": {
                "parent_thread_id": "01900000-0000-7000-8000-000000000001",
                "depth": 1, "agent_path": "/root/judge", "agent_nickname": "Judge", "agent_role": null
            }}}
        }));
        assert_eq!(h.agent.kind, AgentKind::CodexThread);
        assert_eq!(
            h.agent.parent,
            Some(AgentId::codex_thread(
                "01900000-0000-7000-8000-000000000001"
            ))
        );
        assert_eq!(
            h.agent.root,
            AgentId::codex_thread("01900000-0000-7000-8000-000000000001")
        );
        assert_eq!(h.agent.path.as_deref(), Some("/root/judge"));
        assert_eq!(h.agent.name.as_deref(), Some("judge"));
        assert_eq!(h.agent.depth, 1);
    }

    #[test]
    fn forked_child() {
        let h = header(serde_json::json!({
            "id": "01900000-0000-7000-8000-000000000003",
            "session_id": "01900000-0000-7000-8000-000000000001",
            "forked_from_id": "01900000-0000-7000-8000-000000000001",
            "parent_thread_id": "01900000-0000-7000-8000-000000000001",
            "agent_path": "/root/fork"
        }));
        assert_eq!(h.agent.kind, AgentKind::Fork);
    }

    #[test]
    fn legacy_review_subagent() {
        let h = header(serde_json::json!({"id": "t", "source": {"subagent": "review"}}));
        assert_eq!(h.agent.kind, AgentKind::CodexThread);
        assert_eq!(h.agent.agent_type.as_deref(), Some("review"));
    }
}
