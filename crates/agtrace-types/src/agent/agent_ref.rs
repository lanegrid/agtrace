use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::id::{AgentId, Provider};

/// What kind of agent a log file belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentKind {
    /// Root session started by a human / bg job (Claude main transcript, Codex root thread).
    Main,
    /// Claude Agent Teams member: separate top-level transcript with teamName/agentName.
    Teammate,
    /// Claude async/background subagent (`subagents/agent-*.jsonl`, not a fork).
    Subagent,
    /// Child whose context is a copy of the parent's (Claude fork; Codex `forked_from_id`).
    Fork,
    /// Codex child thread (thread_spawn, not forked).
    CodexThread,
}

/// Static identity of an agent. Built from the file header (+ sidecars), refined by the graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentRef {
    pub id: AgentId,
    pub provider: Provider,
    pub kind: AgentKind,
    /// Root of the tree this agent belongs to (== id for roots).
    pub root: AgentId,
    /// Direct parent, if known. None for roots and for not-yet-linked children.
    pub parent: Option<AgentId>,
    /// Native session/thread id of the file owner:
    /// Claude main/teammate: sessionId; Claude subagent: parent sessionId; Codex: thread id.
    pub native_session_id: String,
    /// Native per-agent id: Claude subagent agentId; teammate "name@team"; Codex None.
    pub native_agent_id: Option<String>,
    /// Display name (teammate agentName, Codex agent_path leaf / nickname, ...).
    pub name: Option<String>,
    /// Hierarchical path used for display and message routing (Codex agent_path, ...).
    pub path: Option<String>,
    /// Agent type: Claude subagent_type / agentSetting, Codex agent_role.
    pub agent_type: Option<String>,
    pub team: Option<String>,
    /// Provider call id of the spawning tool call in the parent.
    pub spawn_call_id: Option<String>,
    pub depth: u32,
    pub file: PathBuf,
    pub cwd: Option<PathBuf>,
    pub started_at: Option<DateTime<Utc>>,
}

impl AgentRef {
    /// Minimal root agent reference (kind `Main`, no parent).
    pub fn root(id: AgentId, native_session_id: impl Into<String>, file: PathBuf) -> Self {
        Self {
            provider: id.provider(),
            root: id.clone(),
            id,
            kind: AgentKind::Main,
            parent: None,
            native_session_id: native_session_id.into(),
            native_agent_id: None,
            name: None,
            path: None,
            agent_type: None,
            team: None,
            spawn_call_id: None,
            depth: 0,
            file,
            cwd: None,
            started_at: None,
        }
    }
}
