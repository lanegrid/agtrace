//! Sessions: the top-level units of a workspace (design: `watch` sessions list).
//!
//! A **session** is one top-level tree of the view — a Claude main session or a
//! Codex root thread with everything below it — plus, for Claude, the other
//! transcripts of the same *logical* session, which are folded under it instead of
//! being shown as unrelated roots ([`SessionFold`]):
//!
//! | Fold | Rule |
//! |---|---|
//! | `Continued` | the transcript says `continued-in` → the continuation |
//! | `RuntimeAlias` | another transcript wrote records under this transcript's id as its runtime session id (resume / bg daemon respawn) |
//! | `Stub` | a transcript without any conversation (only local commands such as `/clear`, `/resume`) that carries the name (`agent-name` / `ai-title`) of another session |
//!
//! A live Claude process that has not written a transcript yet (a bg job waiting
//! for work) is a session too ([`Session::has_transcript`] false).
//!
//! **Liveness** ([`SessionState`]):
//! - Claude: live while a process of the session is registered and its pid alive
//!   (busy when the registry says so or an agent of the session is running); without
//!   a registry entry, live only while an agent is running.
//! - Codex: busy while an agent of the thread tree is running (a turn is open); idle
//!   while the tree was written to within [`CODEX_SESSION_LIVE`].
//! - Otherwise ended: recent when the last write is within [`SESSION_RECENT`], else
//!   older.
//!
//! **Order**: busy, idle, recent, older; then the most recently active first.
//!
//! **Name** ([`SessionNameSource`]): the registry name when it is not just an id,
//! else the session's own name (`agent-name`, title; Codex nickname), else an
//! excerpt of the first prompt, else the first slash command, else the short id.

use std::collections::{BTreeMap, HashSet};

use agtrace_types::{AgentAttributeKey, AgentId, AgentKind, Provider};
use chrono::{DateTime, Duration, Utc};
use serde::Serialize;

use super::status::{AgentStatus, RegistryEntry};
use super::timeline::{TimelineItem, one_line};
use super::view::{AgentView, WorkspaceView};

/// Codex tree written to within this window counts as a live (idle) session.
pub const CODEX_SESSION_LIVE: Duration = Duration::minutes(30);
/// Ended sessions written to within this window are "recent" (else "older").
pub const SESSION_RECENT: Duration = Duration::hours(1);
/// Characters kept of a prompt used as a session name.
const NAME_MAX: usize = 60;

/// Why a transcript is shown under another transcript of the same logical session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionFold {
    /// `continued-in` points at the parent.
    Continued,
    /// The parent wrote records under this transcript's id (resumed / respawned).
    RuntimeAlias,
    /// No conversation, and the parent's name (a `/clear` or `/resume` stub).
    Stub,
}

/// Liveness of a session, in sort order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    /// Live and working.
    Busy,
    /// Live, waiting for input.
    Idle,
    /// Ended within [`SESSION_RECENT`].
    Recent,
    /// Ended earlier.
    Older,
}

impl SessionState {
    pub fn is_live(self) -> bool {
        matches!(self, SessionState::Busy | SessionState::Idle)
    }
}

/// Where a session's display name comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionNameSource {
    /// Claude process registry `name`.
    Registry,
    /// `agent-name` / agent name (Codex nickname).
    Name,
    /// `ai-title` / custom title.
    Title,
    /// First prompt excerpt.
    Prompt,
    /// First slash command (a session without prompts).
    Command,
    /// Short id.
    Id,
}

/// One session of the workspace.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Session {
    /// Root agent (for a process without transcript: the id its transcript will have).
    pub root: AgentId,
    pub provider: Provider,
    pub name: String,
    pub name_source: SessionNameSource,
    /// First 8 characters of the session id.
    pub short_id: String,
    /// Background (daemon / job) session.
    pub bg: bool,
    pub state: SessionState,
    /// False for a live process that has not written a transcript yet.
    pub has_transcript: bool,
    /// Agents of the session (root and folded transcripts included).
    pub agents: usize,
    pub running: usize,
    pub idle: usize,
    pub done: usize,
    pub killed: usize,
    pub failed: usize,
    pub started_at: Option<DateTime<Utc>>,
    /// Latest own-log write of any agent of the session (registry update for a
    /// process without transcript).
    pub last_activity: Option<DateTime<Utc>>,
}

impl Session {
    pub fn is_live(&self) -> bool {
        self.state.is_live()
    }
}

impl AgentView {
    /// A Claude main transcript without any conversation: no prompt or task, no
    /// assistant output, no tool call (only local commands and metadata).
    pub fn is_stub(&self) -> bool {
        let d = &self.detail;
        self.agent.provider == Provider::ClaudeCode
            && self.agent.kind == AgentKind::Main
            && d.initial_task.is_none()
            && d.last_message.is_none()
            && d.last_reasoning.is_none()
            && d.totals.tool_calls == 0
    }

    /// `agent-name`, else the title: the name a stub inherits from its session.
    pub(super) fn session_name_key(&self) -> Option<&str> {
        self.display_name()
            .or_else(|| {
                self.attributes
                    .get(&AgentAttributeKey::Title)
                    .map(String::as_str)
            })
            .filter(|s| !s.is_empty())
    }

    /// What decides whether this transcript folds into another session.
    pub(super) fn fold_key(&self) -> (bool, Option<String>) {
        (self.is_stub(), self.session_name_key().map(str::to_string))
    }

    /// First slash command in the timeline (`/resume`).
    pub fn first_command(&self) -> Option<String> {
        self.recent.iter().find_map(|e| match &e.item {
            TimelineItem::SlashCommand { name, .. } => Some(if name.starts_with('/') {
                name.clone()
            } else {
                format!("/{name}")
            }),
            _ => None,
        })
    }

    /// Record `sessionKind: "bg"`.
    fn bg_transcript(&self) -> bool {
        self.attributes
            .get(&AgentAttributeKey::SessionKind)
            .is_some_and(|k| k == "bg")
    }
}

impl WorkspaceView {
    /// Registry entries of a Claude transcript: its own, plus those of the runtime
    /// session ids it wrote under (a resumed / respawned process registers under
    /// its runtime id), unless such an id is itself a known transcript.
    pub(super) fn registry_entries(&self, id: &AgentId) -> Vec<&RegistryEntry> {
        let mut out: Vec<&RegistryEntry> = self
            .registry
            .get(id)
            .map(|m| m.values().collect())
            .unwrap_or_default();
        for alias in self.runtime_aliases.of(id) {
            let aid = AgentId::claude_session(alias);
            if self.agents.contains_key(&aid) {
                continue;
            }
            if let Some(m) = self.registry.get(&aid) {
                out.extend(m.values());
            }
        }
        out
    }

    /// Transcript of the same logical session to show `id` under, and why.
    pub(super) fn same_session_parent_why(&self, id: &AgentId) -> Option<(AgentId, SessionFold)> {
        let v = self.agents.get(id)?;
        if let Some(next) = v.continued_in() {
            let next = AgentId::claude_session(next);
            if next != *id && self.agents.contains_key(&next) {
                return Some((next, SessionFold::Continued));
            }
        }
        if v.agent.provider != Provider::ClaudeCode || v.agent.kind != AgentKind::Main {
            return None;
        }
        if let Some(t) = self.runtime_aliases.get(id.native_session_id())
            && t != id
            && self
                .agents
                .get(t)
                .is_some_and(|x| x.agent.kind == AgentKind::Main)
        {
            return Some((t.clone(), SessionFold::RuntimeAlias));
        }
        if v.is_stub() {
            let key = v.session_name_key()?;
            return self
                .agents
                .values()
                .filter(|o| {
                    o.agent.id != *id
                        && o.agent.provider == Provider::ClaudeCode
                        && o.agent.kind == AgentKind::Main
                        && !o.is_stub()
                        && o.session_name_key() == Some(key)
                })
                .max_by_key(|o| (o.last_activity, o.agent.id.clone()))
                .map(|o| (o.agent.id.clone(), SessionFold::Stub));
        }
        None
    }

    pub(super) fn same_session_parent(&self, id: &AgentId) -> Option<AgentId> {
        self.same_session_parent_why(id).map(|(p, _)| p)
    }

    /// Top-level row (session root) of the tree containing `id`.
    pub fn session_root<'a>(&'a self, id: &'a AgentId) -> &'a AgentId {
        let mut cur = id;
        let mut steps = 0;
        while let Some(p) = self.agents.get(cur).and_then(|a| a.tree_parent.as_ref()) {
            cur = p;
            steps += 1;
            if steps > 64 {
                break;
            }
        }
        cur
    }

    /// Agents of the session rooted at `root` (pre-order, root first).
    pub fn session_agents<'a>(&'a self, root: &'a AgentId) -> Vec<&'a AgentId> {
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        let mut stack = vec![root];
        while let Some(id) = stack.pop() {
            if !seen.insert(id) {
                continue;
            }
            let Some(v) = self.agents.get(id) else {
                continue;
            };
            out.push(id);
            stack.extend(v.children.iter().rev());
        }
        out
    }

    /// Every session, in display order (see the module docs).
    pub fn sessions(&self, now: DateTime<Utc>) -> Vec<Session> {
        let mut out: Vec<Session> = self
            .roots
            .iter()
            .filter_map(|r| self.session(r, now))
            .collect();
        out.extend(self.process_only_sessions());
        out.sort_by(|a, b| {
            a.state
                .cmp(&b.state)
                .then_with(|| b.last_activity.cmp(&a.last_activity))
                .then_with(|| a.root.cmp(&b.root))
        });
        out
    }

    fn session(&self, root: &AgentId, now: DateTime<Utc>) -> Option<Session> {
        let r = self.agents.get(root)?;
        let members = self.session_agents(root);
        let mut s = Session {
            root: root.clone(),
            provider: r.agent.provider,
            name: String::new(),
            name_source: SessionNameSource::Id,
            short_id: short_id(root.native_session_id()),
            bg: false,
            state: SessionState::Older,
            has_transcript: true,
            agents: members.len(),
            running: 0,
            idle: 0,
            done: 0,
            killed: 0,
            failed: 0,
            started_at: r.agent.started_at,
            last_activity: None,
        };
        let mut registry_alive = false;
        let mut registry_busy = false;
        // Latest live registry name among the session's own processes.
        let mut registry_name: Option<(DateTime<Utc>, &str, &AgentId)> = None;
        for id in &members {
            let v = &self.agents[*id];
            match v.status {
                AgentStatus::Running => s.running += 1,
                AgentStatus::Idle => s.idle += 1,
                AgentStatus::Done => s.done += 1,
                AgentStatus::Killed => s.killed += 1,
                AgentStatus::Failed => s.failed += 1,
                AgentStatus::Unknown => {}
            }
            s.last_activity = s.last_activity.max(v.last_activity);
            // Processes of the session itself: the root and folded transcripts.
            if *id == root || v.session_fold.is_some() {
                for e in self.registry_entries(id) {
                    if e.alive {
                        registry_alive = true;
                        registry_busy |= e.status == Some(super::input::ProcessStatus::Busy);
                        s.bg |= e.bg;
                        if let Some(n) = e.name.as_deref()
                            && registry_name.is_none_or(|(t, _, _)| e.updated_at >= t)
                        {
                            registry_name = Some((e.updated_at, n, id));
                        }
                    }
                }
                s.bg |= v.bg_transcript();
            }
        }
        let written_within = |d: Duration| s.last_activity.is_some_and(|t| now - t < d);
        let codex_live = r.agent.provider == Provider::Codex && written_within(CODEX_SESSION_LIVE);
        s.state = if registry_busy || s.running > 0 {
            SessionState::Busy
        } else if registry_alive || codex_live {
            SessionState::Idle
        } else if written_within(SESSION_RECENT) {
            SessionState::Recent
        } else {
            SessionState::Older
        };
        let registry_name = registry_name
            .filter(|(_, n, id)| !self.id_like(id, n) && !self.id_like(root, n))
            .map(|(_, n, _)| n);
        let (name, source) = self.session_name(r, registry_name);
        s.name = name;
        s.name_source = source;
        Some(s)
    }

    /// Display name of the session rooted at `r` (see the module docs);
    /// `registry` is the live registry name of the session's processes.
    fn session_name(&self, r: &AgentView, registry: Option<&str>) -> (String, SessionNameSource) {
        let id = r.id();
        if let Some(name) = registry {
            return (one_line(name, NAME_MAX), SessionNameSource::Registry);
        }
        match r.agent.provider {
            Provider::ClaudeCode => {
                if let Some(n) = r.agent.name.as_deref().or_else(|| r.display_name()) {
                    return (one_line(n, NAME_MAX), SessionNameSource::Name);
                }
                if let Some(t) = r.attributes.get(&AgentAttributeKey::Title) {
                    return (one_line(t, NAME_MAX), SessionNameSource::Title);
                }
            }
            Provider::Codex => {
                // Roots carry the path `/root` as their name: not a name.
                if let Some(n) = r.agent.name.as_deref().filter(|n| !n.starts_with('/')) {
                    return (one_line(n, NAME_MAX), SessionNameSource::Name);
                }
            }
        }
        if let Some(text) = r
            .detail
            .all_instructions()
            .find_map(|i| i.text.as_deref().filter(|t| !t.trim().is_empty()))
        {
            return (one_line(text, NAME_MAX), SessionNameSource::Prompt);
        }
        if let Some(cmd) = r.first_command() {
            let short = short_id(id.native_session_id());
            return (format!("{cmd} · {short}"), SessionNameSource::Command);
        }
        (short_id(id.native_session_id()), SessionNameSource::Id)
    }

    /// A registry name that only repeats an id (Claude names bg jobs by their id).
    fn id_like(&self, id: &AgentId, name: &str) -> bool {
        let name = name.trim();
        if name.is_empty() {
            return true;
        }
        let prefix_of = |sid: &str| sid.starts_with(name) || name.starts_with(sid);
        prefix_of(id.native_session_id())
            || self.runtime_aliases.of(id).any(prefix_of)
            || (name.len() == 8 && name.chars().all(|c| c.is_ascii_hexdigit()))
    }

    /// Live registered Claude processes without a known transcript.
    fn process_only_sessions(&self) -> Vec<Session> {
        let mut out = Vec::new();
        for (id, entries) in &self.registry {
            if self.agents.contains_key(id)
                || self.runtime_aliases.get(id.native_session_id()).is_some()
            {
                continue;
            }
            let Some(e) = alive_latest(entries) else {
                continue;
            };
            let name = e
                .name
                .as_deref()
                .filter(|n| !self.id_like(id, n))
                .map(|n| (n.to_string(), SessionNameSource::Registry));
            let short = short_id(id.native_session_id());
            let (name, name_source) = name.unwrap_or((short.clone(), SessionNameSource::Id));
            out.push(Session {
                root: id.clone(),
                provider: Provider::ClaudeCode,
                name,
                name_source,
                short_id: short,
                bg: e.bg,
                state: if e.status == Some(super::input::ProcessStatus::Busy) {
                    SessionState::Busy
                } else {
                    SessionState::Idle
                },
                has_transcript: false,
                agents: 0,
                running: 0,
                idle: 0,
                done: 0,
                killed: 0,
                failed: 0,
                started_at: None,
                last_activity: Some(e.updated_at),
            });
        }
        out
    }
}

fn alive_latest(entries: &BTreeMap<u32, RegistryEntry>) -> Option<&RegistryEntry> {
    entries
        .values()
        .filter(|e| e.alive)
        .max_by_key(|e| e.updated_at)
}

fn short_id(sid: &str) -> String {
    sid.chars().take(8).collect()
}
