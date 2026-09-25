//! Inputs of the workspace fold (design §4.2). Produced by the runtime workspace
//! watcher, consumed by [`super::WorkspaceView::apply`].

use agtrace_types::{AgentEvent, AgentId, AgentRef, ParseDiagnostics};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// One observation from the workspace watcher.
#[derive(Debug, Clone)]
pub enum WorkspaceEvent {
    /// A new agent file was found (header read).
    AgentDiscovered(AgentRef),
    /// Header / sidecar information of a known agent changed (parent, name, ...).
    AgentUpdated(AgentRef),
    /// New events decoded from the agent's own file, in file order.
    /// `reset` = the file was truncated or replaced: all state derived from this
    /// agent's own log is discarded before `events` are applied.
    Events {
        agent: AgentId,
        events: Vec<AgentEvent>,
        reset: bool,
    },
    /// Registry / team config changes.
    SideState(SideStateUpdate),
    /// Cumulative decoder diagnostics of the agent's file (replaces the previous value).
    Diagnostics {
        agent: AgentId,
        diagnostics: ParseDiagnostics,
    },
    /// Watcher-level error (I/O, permission, ...), shown in the status bar.
    Error(String),
}

/// Claude process registry status (`~/.claude/sessions/<pid>.json` `status`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessStatus {
    Busy,
    Idle,
}

/// One `members[]` entry of `~/.claude/teams/<team>/config.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamMember {
    /// Member name (`team-lead`, `audit-A`, ...).
    pub name: String,
    pub agent_type: Option<String>,
    /// Model string as written in the config (may carry a `[1m]` suffix).
    pub model: Option<String>,
    /// `isActive`; `None` when absent (the lead entry has no `isActive`).
    pub is_active: Option<bool>,
}

/// State kept outside the agent logs.
#[derive(Debug, Clone, PartialEq)]
pub enum SideStateUpdate {
    /// `~/.claude/sessions/<pid>.json`: `alive` = entry present and pid alive.
    /// An entry that disappeared is reported once with `alive: false`.
    ClaudeProcess {
        session_id: String,
        pid: u32,
        alive: bool,
        status: Option<ProcessStatus>,
        name: Option<String>,
        /// Background (daemon / job) session (registry `kind: "bg"`).
        bg: bool,
        updated_at: DateTime<Utc>,
    },
    /// `~/.claude/teams/<team>/config.json` (full member list, replaces the previous one).
    ClaudeTeam {
        team: String,
        lead_session_id: String,
        members: Vec<TeamMember>,
    },
    /// `subagents/agent-<aid>.meta.json` of a Claude subagent / fork.
    /// `stopped_by_user` ⇒ Killed (parent-side terminal); `model` is a context hint.
    ClaudeSubagentMeta {
        agent: AgentId,
        stopped_by_user: bool,
        model: Option<String>,
    },
}
