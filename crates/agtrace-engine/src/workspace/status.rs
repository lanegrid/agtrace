//! Agent status derivation (design §4.3 status table).
//!
//! Every agent keeps the raw *signals* per priority layer; the effective status is
//! recomputed from them (plus `now` for staleness) instead of being mutated in
//! place. That makes the result independent of the order in which files are
//! attached: parent-side terminals and own-log activity are compared by timestamp.
//!
//! | Priority | Signal |
//! |---|---|
//! | 1 parent-side terminal | `terminal` (sticky until own-log activity after it) + parent `AllBackgroundKilled` |
//! | 3 registry (Claude main/teammate) | wins while the pid is alive (§10.7); dead ⇒ Done |
//! | 2 own log | latest of `own` (file order) and `remote` (parent-side non-terminal) |
//! | 4 staleness | no write for a while ⇒ Idle / Done (Codex child: only under a finished parent) |
//!
//! (Registry is checked before the own log because §10.7 makes it win while alive.)

use std::collections::BTreeMap;

use agtrace_types::{AgentKind, Provider};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use super::input::ProcessStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Running,
    Idle,
    Done,
    Killed,
    Failed,
    Unknown,
}

impl AgentStatus {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            AgentStatus::Done | AgentStatus::Killed | AgentStatus::Failed
        )
    }
}

/// Which rule produced the current status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusSource {
    /// Nothing known yet.
    None,
    /// Another agent's log (task-notification, SubAgentActivity, idle_notification, ...)
    /// or team config / subagent meta.
    ParentEvent,
    OwnLog,
    Registry,
    Staleness,
}

/// Staleness thresholds (design §4.3, priority 4).
pub const CLAUDE_RUNNING_STALE: Duration = Duration::minutes(10);
pub const SUBAGENT_RUNNING_STALE: Duration = Duration::minutes(10);
pub const CODEX_RUNNING_STALE: Duration = Duration::minutes(30);
pub const IDLE_TO_DONE: Duration = Duration::hours(2);

/// Where a parent-side terminal came from (team config terminals are revocable).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TerminalOrigin {
    Event,
    TeamConfig,
    SubagentMeta,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Terminal {
    pub status: AgentStatus,
    pub at: DateTime<Utc>,
    pub origin: TerminalOrigin,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RegistryEntry {
    pub alive: bool,
    pub status: Option<ProcessStatus>,
    pub name: Option<String>,
    pub bg: bool,
    pub updated_at: DateTime<Utc>,
}

/// Raw per-layer status signals of one agent.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct StatusSignals {
    /// Priority 1: latest parent-side terminal.
    pub terminal: Option<Terminal>,
    /// Priority 2, own log (file order; always overwritten).
    pub own: Option<(AgentStatus, DateTime<Utc>)>,
    /// Priority 2, parent-side non-terminal (idle_notification, interacted, interrupted).
    pub remote: Option<(AgentStatus, DateTime<Utc>)>,
    /// Timestamp of the latest *activity* event in the agent's own log.
    pub last_active: Option<DateTime<Utc>>,
    /// Timestamp of the latest start of *new work* in the agent's own log (a
    /// prompt, an incoming task/message, or a turn start). Only this re-opens a
    /// parent-side terminal: records the agent writes while finishing (final
    /// text, usage, turn end) must not.
    pub last_started: Option<DateTime<Utc>>,
    /// Timestamp of the latest event of any kind in the agent's own log ("last write").
    pub last_write: Option<DateTime<Utc>>,
    /// This agent's log said "all background agents killed" at this time.
    pub all_background_killed_at: Option<DateTime<Utc>>,
}

impl StatusSignals {
    /// Record a parent-side terminal; the latest one (by timestamp) wins.
    pub fn set_terminal(&mut self, status: AgentStatus, at: DateTime<Utc>, origin: TerminalOrigin) {
        if self.terminal.is_none_or(|t| at >= t.at) {
            self.terminal = Some(Terminal { status, at, origin });
        }
    }

    pub fn set_remote(&mut self, status: AgentStatus, at: DateTime<Utc>) {
        if self.remote.is_none_or(|(_, t)| at >= t) {
            self.remote = Some((status, at));
        }
    }

    /// Own-log activity strictly after `t`.
    fn active_after(&self, t: DateTime<Utc>) -> bool {
        self.last_active.is_some_and(|a| a > t)
    }

    /// Terminal still in force (not superseded by own activity).
    fn live_terminal(&self) -> Option<Terminal> {
        self.terminal
            .filter(|t| self.last_started.is_none_or(|s| s <= t.at))
    }

    /// Priority-2 status: latest of own log and parent-side non-terminal signals.
    fn log_status(&self) -> Option<(AgentStatus, StatusSource)> {
        match (self.own, self.remote) {
            (Some((o, ot)), Some((r, rt))) => Some(if rt >= ot {
                (r, StatusSource::ParentEvent)
            } else {
                (o, StatusSource::OwnLog)
            }),
            (Some((o, _)), None) => Some((o, StatusSource::OwnLog)),
            (None, Some((r, _))) => Some((r, StatusSource::ParentEvent)),
            (None, None) => None,
        }
    }
}

/// What the derivation needs to know about the agent's (tree) parent.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ParentContext {
    /// None = no parent in the view ("gone").
    pub status: Option<AgentStatus>,
    pub all_background_killed_at: Option<DateTime<Utc>>,
}

/// Static facts about the agent the derivation depends on.
#[derive(Debug, Clone, Copy)]
pub(crate) struct AgentFacts {
    pub provider: Provider,
    pub kind: AgentKind,
    /// Root agent of its provider tree (`AgentKind::Main`).
    pub is_root: bool,
}

/// Effective status of an agent from its signals.
pub(crate) fn derive_status(
    facts: AgentFacts,
    s: &StatusSignals,
    registry: &BTreeMap<u32, RegistryEntry>,
    parent: ParentContext,
    now: DateTime<Utc>,
) -> (AgentStatus, StatusSource) {
    let claude = facts.provider == Provider::ClaudeCode;
    let claude_session = claude && matches!(facts.kind, AgentKind::Main | AgentKind::Teammate);
    let claude_subagent = claude && !claude_session;

    // 1. Parent-side terminal (sticky unless the own log moved on).
    if let Some(t) = s.live_terminal() {
        return (t.status, StatusSource::ParentEvent);
    }

    let log = s.log_status();

    // 1b. The parent killed all background agents while this one was still running.
    if claude_subagent
        && let Some(killed_at) = parent.all_background_killed_at
        && matches!(log, Some((AgentStatus::Running, _)))
        && !s.active_after(killed_at)
    {
        return (AgentStatus::Killed, StatusSource::ParentEvent);
    }

    // 3. Registry (Claude sessions only; wins while the pid is alive, §10.7).
    if claude_session && !registry.is_empty() {
        let alive = registry
            .values()
            .filter(|e| e.alive)
            .max_by_key(|e| e.updated_at);
        match alive {
            Some(entry) => {
                return match entry.status {
                    Some(ProcessStatus::Busy) => (AgentStatus::Running, StatusSource::Registry),
                    Some(ProcessStatus::Idle) => (AgentStatus::Idle, StatusSource::Registry),
                    // Alive but status unknown: own log decides, no staleness.
                    None => log.unwrap_or((AgentStatus::Unknown, StatusSource::None)),
                };
            }
            None => {
                // Entry gone / pid dead ⇒ Done, unless the session was resumed since.
                let gone_at = registry.values().map(|e| e.updated_at).max();
                if gone_at.is_some_and(|g| !s.active_after(g)) {
                    return (AgentStatus::Done, StatusSource::Registry);
                }
            }
        }
    }

    // 2. Own log / parent-side non-terminal.
    let Some((status, source)) = log else {
        return (AgentStatus::Unknown, StatusSource::None);
    };

    // 4. Staleness.
    let stale = |d: Duration| s.last_write.is_some_and(|w| now - w >= d);
    let live = matches!(status, AgentStatus::Running | AgentStatus::Idle);
    if claude_session && registry.is_empty() {
        if live && stale(IDLE_TO_DONE) {
            return (AgentStatus::Done, StatusSource::Staleness);
        }
        if status == AgentStatus::Running && stale(CLAUDE_RUNNING_STALE) {
            return (AgentStatus::Idle, StatusSource::Staleness);
        }
    } else if claude_subagent {
        let parent_gone = parent.status.is_none_or(AgentStatus::is_terminal);
        if live && parent_gone && stale(SUBAGENT_RUNNING_STALE) {
            return (AgentStatus::Done, StatusSource::Staleness);
        }
    } else if facts.provider == Provider::Codex {
        // A child is only ever resumed through its parent: once the parent is
        // finished (or gone), an idle child is finished too, like the root.
        let parent_gone = parent.status.is_none_or(AgentStatus::is_terminal);
        if live && (facts.is_root || parent_gone) && stale(IDLE_TO_DONE) {
            return (AgentStatus::Done, StatusSource::Staleness);
        }
        if status == AgentStatus::Running && stale(CODEX_RUNNING_STALE) {
            return (AgentStatus::Idle, StatusSource::Staleness);
        }
    }
    (status, source)
}
