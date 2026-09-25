//! Synchronous watcher state: discovery ticks and poll ticks produce
//! [`WorkspaceEvent`]s. No threads, no sleeping — the loop in `mod.rs` drives it,
//! tests drive it directly.

use super::liveness::pid_alive;
use super::{WatchRoots, WatchScope};
use crate::tail::{FileCursor, TailOutcome, provider_for};
use agtrace_engine::workspace::{
    ProcessStatus, RuntimeAliases, SideStateUpdate, TeamMember, WorkspaceEvent, team_lead_agent,
};
use agtrace_providers::claude::{
    ClaudeProcessEntry, ClaudeProcessStatus, ClaudeTeamConfig, is_agent_file_path, project_dirs,
    read_process_entry, read_subagent_meta, read_team_config, session_registry_paths,
    team_config_paths,
};
use agtrace_providers::{DecodeOptions, FileHeader, Provider, ProviderId};
use agtrace_types::{
    AgentAttributeKey, AgentEvent, AgentHandle, AgentId, AgentKind, AgentRef, EventPayload,
};
use chrono::{DateTime, Local, NaiveDate, Utc};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

/// Upper bound of list/accept rounds per discovery tick (roots, then their
/// subagent dirs, then children linked to those, ...).
const MAX_DISCOVERY_ROUNDS: usize = 4;

/// How often Codex date dirs older than yesterday are re-listed. New rollouts are
/// always created in today's dir, so older dirs only matter for files that already
/// exist (initial discovery) or that become active again (project window).
const CODEX_HISTORY_RELIST: Duration = Duration::from_secs(30);

/// Order in which ready candidates are tracked: parents first, then by start time.
type AcceptOrder = (bool, Option<DateTime<Utc>>);

/// A file seen by discovery but not tracked (yet).
struct Candidate {
    provider: ProviderId,
    header: Option<FileHeader>,
    mtime: Option<SystemTime>,
    /// File length at the last header attempt (`read_header` is retried only when
    /// the file changed).
    header_tried_at: Option<u64>,
}

/// A tracked agent file.
struct Tracked {
    /// Latest agent ref sent to the consumer.
    agent: AgentRef,
    cursor: FileCursor,
    /// Header agent of the cursor at the previous poll (refinement detection).
    cursor_agent: Option<AgentRef>,
    /// Events of this file were delivered before (a later reset must say so).
    delivered: bool,
    /// (invalid_json, schema_mismatch, unknown kinds) at the last Diagnostics event.
    diag_sig: (u64, u64, u64),
    last_error: Option<String>,
    /// Claude subagents: mtime of the `.meta.json` sidecar at the last read.
    meta_mtime: Option<Option<SystemTime>>,
}

struct RegistryFile {
    mtime: Option<SystemTime>,
    entry: Option<ClaudeProcessEntry>,
    alive: bool,
}

struct TeamFile {
    mtime: Option<SystemTime>,
    config: Option<ClaudeTeamConfig>,
}

pub(crate) struct WatcherState {
    scope: WatchScope,
    roots: WatchRoots,
    claude: Arc<dyn Provider>,
    codex: Arc<dyn Provider>,
    candidates: HashMap<PathBuf, Candidate>,
    tracked: BTreeMap<PathBuf, Tracked>,
    agents: HashMap<AgentId, PathBuf>,
    registry: HashMap<PathBuf, RegistryFile>,
    registry_emitted: HashMap<String, SideStateUpdate>,
    teams: HashMap<PathBuf, TeamFile>,
    teams_emitted: HashMap<String, SideStateUpdate>,
    /// Team names mentioned by tracked agents (headers, attributes, spawns).
    referenced_teams: HashSet<String>,
    /// Runtime session ids of tracked Claude transcripts (a team config's
    /// `leadSessionId` may be one of them).
    runtime_aliases: RuntimeAliases,
    /// Root scope, Claude target: the project dir holding `<sid>.jsonl`.
    root_claude_dir: Option<PathBuf>,
    /// Root scope, Codex target: local date of the dir holding the root rollout
    /// (`None` until looked up, `Some(None)` if not found).
    codex_root_day: Option<Option<NaiveDate>>,
    /// When the Codex date dirs older than yesterday are listed next.
    codex_history_due: Option<SystemTime>,
}

impl WatcherState {
    pub(crate) fn new(scope: WatchScope, roots: WatchRoots) -> Self {
        Self {
            scope,
            roots,
            claude: provider_for(ProviderId::ClaudeCode),
            codex: provider_for(ProviderId::Codex),
            candidates: HashMap::new(),
            tracked: BTreeMap::new(),
            agents: HashMap::new(),
            registry: HashMap::new(),
            registry_emitted: HashMap::new(),
            teams: HashMap::new(),
            teams_emitted: HashMap::new(),
            referenced_teams: HashSet::new(),
            runtime_aliases: RuntimeAliases::new(),
            root_claude_dir: None,
            codex_root_day: None,
            codex_history_due: None,
        }
    }

    // ------------------------------------------------------------------
    // Ticks
    // ------------------------------------------------------------------

    /// Re-list the bounded discovery set, track new agents in scope (with their
    /// initial tail), and report side-state changes.
    pub(crate) fn discovery_tick(&mut self, now: SystemTime) -> Vec<WorkspaceEvent> {
        let mut out = Vec::new();
        self.refresh_registry();
        self.refresh_teams();
        let history = self
            .codex_history_due
            .is_none_or(|due| now >= due || now + CODEX_HISTORY_RELIST < due);
        if history {
            self.codex_history_due = Some(now + CODEX_HISTORY_RELIST);
        }
        for round in 0..MAX_DISCOVERY_ROUNDS {
            for (path, provider) in self.list_files(now, history && round == 0) {
                self.update_candidate(path, provider);
            }
            if self.accept_candidates(now, &mut out) == 0 {
                break;
            }
        }
        // Force a poll of every tracked file once per tick (identity changes that keep
        // the size, stale partial lines).
        let paths: Vec<PathBuf> = self.tracked.keys().cloned().collect();
        for path in &paths {
            self.poll_file(path, &mut out);
        }
        self.refresh_subagent_meta(&mut out);
        self.emit_teams(&mut out);
        self.emit_registry(now, &mut out);
        out
    }

    /// `stat` every tracked file and poll those whose size changed (or that hold
    /// an incomplete line).
    pub(crate) fn poll_tick(&mut self) -> Vec<WorkspaceEvent> {
        let mut out = Vec::new();
        let due: Vec<PathBuf> = self
            .tracked
            .iter()
            .filter(|(path, t)| {
                std::fs::metadata(path)
                    .is_ok_and(|m| m.len() != t.cursor.read_end() || t.cursor.has_partial())
            })
            .map(|(path, _)| path.clone())
            .collect();
        for path in &due {
            self.poll_file(path, &mut out);
        }
        out
    }

    // ------------------------------------------------------------------
    // Discovery
    // ------------------------------------------------------------------

    /// Files of the discovery set. `history` adds the Codex date dirs older than
    /// yesterday that the scope reaches.
    fn list_files(&mut self, now: SystemTime, history: bool) -> Vec<(PathBuf, ProviderId)> {
        let mut files = Vec::new();
        for dir in self.claude_project_dirs() {
            for path in list_dir(&dir) {
                if path.is_file() && is_agent_file_path(&path) {
                    files.push((path, ProviderId::ClaudeCode));
                }
            }
        }
        // Subagent dirs of tracked Claude sessions only (bounded by what is watched).
        let subagent_dirs: Vec<PathBuf> = self
            .tracked
            .iter()
            .filter(|(_, t)| {
                t.agent.provider == ProviderId::ClaudeCode && t.agent.native_agent_id.is_none()
            })
            .filter_map(|(path, t)| {
                Some(
                    path.parent()?
                        .join(&t.agent.native_session_id)
                        .join("subagents"),
                )
            })
            .collect();
        for dir in subagent_dirs {
            for path in list_dir(&dir) {
                if path.is_file() && is_agent_file_path(&path) {
                    files.push((path, ProviderId::ClaudeCode));
                }
            }
        }
        for dir in self.codex_dirs(now, history) {
            for path in list_dir(&dir) {
                if path.is_file() && self.codex.probe(&path) {
                    files.push((path, ProviderId::Codex));
                }
            }
        }
        files
    }

    fn claude_project_dirs(&mut self) -> Vec<PathBuf> {
        let Some(projects) = self.roots.claude_projects.clone() else {
            return Vec::new();
        };
        match &self.scope {
            WatchScope::Project { root, .. } => project_dirs(&projects, root),
            WatchScope::Root(target) if target.provider() == ProviderId::ClaudeCode => {
                if self.root_claude_dir.is_none() {
                    let file = format!("{}.jsonl", target.native_session_id());
                    self.root_claude_dir = list_dir(&projects)
                        .into_iter()
                        .find(|dir| dir.join(&file).is_file());
                }
                self.root_claude_dir.iter().cloned().collect()
            }
            WatchScope::Root(_) => Vec::new(),
        }
    }

    /// `<sessions>/<today>` and `<yesterday>` (local dates); with `history`, also the
    /// older date dirs the scope reaches (see [`codex_history_range`]).
    fn codex_dirs(&mut self, now: SystemTime, history: bool) -> Vec<PathBuf> {
        let Some(sessions) = self.roots.codex_sessions.clone() else {
            return Vec::new();
        };
        let today = DateTime::<Local>::from(now).date_naive();
        let mut days: Vec<NaiveDate> = [Some(today), today.pred_opt()]
            .into_iter()
            .flatten()
            .collect();
        if history {
            let first = match &self.scope {
                WatchScope::Project { since, .. } => {
                    let start = now.checked_sub(*since).unwrap_or(SystemTime::UNIX_EPOCH);
                    Some(DateTime::<Local>::from(start).date_naive())
                }
                WatchScope::Root(target) if target.provider() == ProviderId::Codex => {
                    *self.codex_root_day.get_or_insert_with(|| {
                        // Recent dirs first; the tree walk is a one-off for old roots.
                        let suffix = format!("-{}.jsonl", target.native_session_id());
                        let has_root = |dir: &Path| {
                            list_dir(dir)
                                .iter()
                                .any(|p| p.to_string_lossy().ends_with(&suffix))
                        };
                        days.iter()
                            .copied()
                            .find(|d| has_root(&date_dir(&sessions, *d)))
                            .or_else(|| {
                                find_file_dir(&sessions, &suffix)
                                    .and_then(|dir| date_of_dir(&sessions, &dir))
                            })
                    })
                }
                WatchScope::Root(_) => None,
            };
            if let Some(first) = first {
                days.extend(codex_history_range(first, today));
            }
        }
        days.into_iter().map(|d| date_dir(&sessions, d)).collect()
    }

    fn update_candidate(&mut self, path: PathBuf, provider: ProviderId) {
        if self.tracked.contains_key(&path) {
            return;
        }
        let Ok(meta) = std::fs::metadata(&path) else {
            self.candidates.remove(&path);
            return;
        };
        let len = meta.len();
        let candidate = self.candidates.entry(path.clone()).or_insert(Candidate {
            provider,
            header: None,
            mtime: None,
            header_tried_at: None,
        });
        candidate.mtime = meta.modified().ok();
        if candidate.header.is_none() && candidate.header_tried_at != Some(len) {
            let p = match provider {
                ProviderId::ClaudeCode => &self.claude,
                ProviderId::Codex => &self.codex,
            };
            candidate.header = p.read_header(&path).ok().flatten();
            candidate.header_tried_at = Some(len);
        }
    }

    /// Track every candidate in scope, parents before children, until nothing new
    /// qualifies. Returns the number of newly tracked files.
    fn accept_candidates(&mut self, now: SystemTime, out: &mut Vec<WorkspaceEvent>) -> usize {
        let mut accepted = 0;
        loop {
            let mut ready: Vec<(PathBuf, AcceptOrder)> = self
                .candidates
                .iter()
                .filter_map(|(path, c)| {
                    let header = c.header.as_ref()?;
                    self.in_scope(header, c.mtime, now).then(|| {
                        (
                            path.clone(),
                            (header.agent.parent.is_some(), header.agent.started_at),
                        )
                    })
                })
                .collect();
            if ready.is_empty() {
                return accepted;
            }
            ready.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
            for (path, _) in ready {
                let Some(candidate) = self.candidates.remove(&path) else {
                    continue;
                };
                let Some(header) = candidate.header else {
                    continue;
                };
                if self.agents.contains_key(&header.agent.id) {
                    continue; // same agent seen under another path (alias dir)
                }
                self.track(path, candidate.provider, header, out);
                accepted += 1;
            }
        }
    }

    fn in_scope(&self, header: &FileHeader, mtime: Option<SystemTime>, now: SystemTime) -> bool {
        let agent = &header.agent;
        // Descendants of tracked agents are always in scope.
        if agent
            .parent
            .as_ref()
            .is_some_and(|p| self.agents.contains_key(p))
        {
            return true;
        }
        if agent.root != agent.id && self.agents.contains_key(&agent.root) {
            return true;
        }
        match &self.scope {
            WatchScope::Project { root, since } => {
                let cwd = header.project_cwd.as_ref().or(agent.cwd.as_ref());
                if !cwd.is_some_and(|c| c.starts_with(root)) {
                    return false;
                }
                if agent.kind == AgentKind::Teammate
                    && self
                        .team_lead_agent(agent.team.as_deref())
                        .is_some_and(|lead| self.agents.contains_key(&lead))
                {
                    return true;
                }
                let cutoff = now.checked_sub(*since).unwrap_or(SystemTime::UNIX_EPOCH);
                if mtime.is_some_and(|m| m >= cutoff) {
                    return true;
                }
                agent.provider == ProviderId::ClaudeCode
                    && self.registry_live(&agent.native_session_id)
            }
            WatchScope::Root(target) => {
                agent.id == *target
                    || agent.root == *target
                    || (agent.kind == AgentKind::Teammate
                        && target.provider() == ProviderId::ClaudeCode
                        && self.team_lead_agent(agent.team.as_deref()).as_ref() == Some(target))
            }
        }
    }

    fn track(
        &mut self,
        path: PathBuf,
        provider: ProviderId,
        header: FileHeader,
        out: &mut Vec<WorkspaceEvent>,
    ) {
        let cursor = FileCursor::new(
            provider_for(provider),
            path.clone(),
            DecodeOptions::default(),
        );
        if let Some(team) = &header.agent.team {
            self.referenced_teams.insert(team.clone());
        }
        out.push(WorkspaceEvent::AgentDiscovered(header.agent.clone()));
        self.agents.insert(header.agent.id.clone(), path.clone());
        self.tracked.insert(
            path.clone(),
            Tracked {
                agent: header.agent,
                cursor,
                cursor_agent: None,
                delivered: false,
                diag_sig: (0, 0, 0),
                last_error: None,
                meta_mtime: None,
            },
        );
        self.refresh_meta_of(&path, out);
        self.poll_file(&path, out);
    }

    // ------------------------------------------------------------------
    // Tailing
    // ------------------------------------------------------------------

    fn poll_file(&mut self, path: &Path, out: &mut Vec<WorkspaceEvent>) {
        let Some(t) = self.tracked.get_mut(path) else {
            return;
        };
        let outcome = match t.cursor.poll() {
            Ok(outcome) => {
                t.last_error = None;
                outcome
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return,
            Err(e) => {
                let msg = format!("{}: {e}", path.display());
                if t.last_error.as_deref() != Some(msg.as_str()) {
                    out.push(WorkspaceEvent::Error(msg.clone()));
                    t.last_error = Some(msg);
                }
                return;
            }
        };
        let (events, reset) = match outcome {
            TailOutcome::Unchanged => return,
            TailOutcome::Appended(events) => (events, false),
            TailOutcome::Reset(events) => (events, t.delivered),
        };

        // Header refinement (the cursor re-reads the header while the file is young).
        let mut renamed: Option<(AgentId, AgentId)> = None;
        if let Some(header) = t.cursor.header() {
            let current = &header.agent;
            if t.cursor_agent.as_ref() != Some(current) {
                t.cursor_agent = Some(current.clone());
                if *current != t.agent {
                    if current.id != t.agent.id {
                        renamed = Some((t.agent.id.clone(), current.id.clone()));
                        out.push(WorkspaceEvent::AgentDiscovered(current.clone()));
                    } else {
                        out.push(WorkspaceEvent::AgentUpdated(current.clone()));
                    }
                    t.agent = current.clone();
                }
            }
        }
        let agent = t.agent.id.clone();
        if !events.is_empty() || reset {
            out.push(WorkspaceEvent::Events {
                agent: agent.clone(),
                events: events.clone(),
                reset,
            });
            t.delivered = true;
        }
        if let Some(d) = t.cursor.diagnostics() {
            let sig = (
                d.invalid_json,
                d.schema_mismatch_total(),
                d.unknown_kinds.values().sum::<u64>(),
            );
            if sig != t.diag_sig {
                t.diag_sig = sig;
                out.push(WorkspaceEvent::Diagnostics {
                    agent,
                    diagnostics: d.clone(),
                });
            }
        }
        if let Some(team) = t.agent.team.clone() {
            self.referenced_teams.insert(team);
        }
        if let Some((old, new)) = renamed {
            self.agents.remove(&old);
            self.agents.insert(new, path.to_path_buf());
        }
        self.note_team_refs(&events);
    }

    fn note_team_refs(&mut self, events: &[AgentEvent]) {
        for event in events {
            self.runtime_aliases.record(event);
            match &event.payload {
                EventPayload::AgentAttribute(a) if a.key == AgentAttributeKey::TeamName => {
                    self.referenced_teams.insert(a.value.clone());
                }
                EventPayload::AgentSpawn(s) => {
                    if let AgentHandle::TeamMember {
                        team: Some(team), ..
                    } = &s.child
                    {
                        self.referenced_teams.insert(team.clone());
                    }
                }
                _ => {}
            }
        }
    }

    // ------------------------------------------------------------------
    // Side state
    // ------------------------------------------------------------------

    fn refresh_subagent_meta(&mut self, out: &mut Vec<WorkspaceEvent>) {
        let paths: Vec<PathBuf> = self
            .tracked
            .iter()
            .filter(|(_, t)| t.agent.id.is_claude_subagent())
            .map(|(p, _)| p.clone())
            .collect();
        for path in paths {
            self.refresh_meta_of(&path, out);
        }
    }

    /// Re-read `agent-<aid>.meta.json` when it appeared or changed: report
    /// `ClaudeSubagentMeta` and any header change it causes.
    fn refresh_meta_of(&mut self, path: &Path, out: &mut Vec<WorkspaceEvent>) {
        let Some(t) = self.tracked.get_mut(path) else {
            return;
        };
        if !t.agent.id.is_claude_subagent() {
            return;
        }
        let meta_path = path.with_extension("meta.json");
        let mtime = std::fs::metadata(&meta_path)
            .ok()
            .and_then(|m| m.modified().ok());
        if t.meta_mtime == Some(mtime) {
            return;
        }
        let first = t.meta_mtime.is_none();
        t.meta_mtime = Some(mtime);
        let Some(meta) = read_subagent_meta(path) else {
            return;
        };
        out.push(WorkspaceEvent::SideState(
            SideStateUpdate::ClaudeSubagentMeta {
                agent: t.agent.id.clone(),
                stopped_by_user: meta.stopped_by_user,
                model: meta.model.clone(),
            },
        ));
        if !first
            && let Ok(Some(header)) = self.claude.read_header(path)
            && header.agent.id == t.agent.id
            && header.agent != t.agent
        {
            t.agent = header.agent.clone();
            out.push(WorkspaceEvent::AgentUpdated(header.agent));
        }
    }

    fn refresh_registry(&mut self) {
        let Some(home) = self.roots.claude_home.clone() else {
            return;
        };
        let paths = session_registry_paths(&home);
        self.registry.retain(|p, _| paths.contains(p));
        for path in paths {
            let mtime = std::fs::metadata(&path)
                .ok()
                .and_then(|m| m.modified().ok());
            let file = self.registry.entry(path.clone()).or_insert(RegistryFile {
                mtime: None,
                entry: None,
                alive: false,
            });
            if file.entry.is_none() || file.mtime != mtime {
                file.entry = read_process_entry(&path);
                file.mtime = mtime;
            }
            file.alive = file.entry.as_ref().is_some_and(|e| pid_alive(e.pid));
        }
    }

    fn registry_live(&self, session_id: &str) -> bool {
        self.registry
            .values()
            .any(|f| f.alive && f.entry.as_ref().is_some_and(|e| e.session_id == session_id))
    }

    /// Best registry entry per session: alive first, then most recently updated.
    fn registry_by_session(&self) -> HashMap<&str, (&RegistryFile, &ClaudeProcessEntry)> {
        let mut best: HashMap<&str, (&RegistryFile, &ClaudeProcessEntry)> = HashMap::new();
        for file in self.registry.values() {
            let Some(entry) = &file.entry else { continue };
            let key = (file.alive, entry.updated_at.unwrap_or(0));
            match best.get(entry.session_id.as_str()) {
                Some((f, e)) if (f.alive, e.updated_at.unwrap_or(0)) >= key => {}
                _ => {
                    best.insert(entry.session_id.as_str(), (file, entry));
                }
            }
        }
        best
    }

    fn emit_registry(&mut self, now: SystemTime, out: &mut Vec<WorkspaceEvent>) {
        if self.roots.claude_home.is_none() {
            return;
        }
        let sessions: Vec<String> = self
            .tracked
            .values()
            .filter(|t| {
                t.agent.provider == ProviderId::ClaudeCode && t.agent.native_agent_id.is_none()
            })
            .map(|t| t.agent.native_session_id.clone())
            .collect();
        let best = self.registry_by_session();
        let mut updates = Vec::new();
        for sid in sessions {
            let update = match best.get(sid.as_str()) {
                Some((file, entry)) => Some(SideStateUpdate::ClaudeProcess {
                    session_id: sid.clone(),
                    pid: entry.pid,
                    alive: file.alive,
                    status: match entry.status {
                        Some(ClaudeProcessStatus::Busy) => Some(ProcessStatus::Busy),
                        Some(ClaudeProcessStatus::Idle) => Some(ProcessStatus::Idle),
                        _ => None,
                    },
                    name: entry.name.clone(),
                    updated_at: entry
                        .updated_at
                        .and_then(DateTime::<Utc>::from_timestamp_millis)
                        .or(file.mtime.map(DateTime::<Utc>::from))
                        .unwrap_or_else(|| DateTime::<Utc>::from(now)),
                }),
                // Entry disappeared: report it dead once.
                None => match self.registry_emitted.get(&sid) {
                    Some(SideStateUpdate::ClaudeProcess {
                        pid,
                        alive: true,
                        status,
                        name,
                        ..
                    }) => Some(SideStateUpdate::ClaudeProcess {
                        session_id: sid.clone(),
                        pid: *pid,
                        alive: false,
                        status: *status,
                        name: name.clone(),
                        updated_at: DateTime::<Utc>::from(now),
                    }),
                    _ => None,
                },
            };
            if let Some(update) = update {
                updates.push((sid, update));
            }
        }
        for (sid, update) in updates {
            let changed = match (self.registry_emitted.get(&sid), &update) {
                (
                    Some(SideStateUpdate::ClaudeProcess {
                        pid: p0,
                        alive: a0,
                        status: s0,
                        name: n0,
                        ..
                    }),
                    SideStateUpdate::ClaudeProcess {
                        pid,
                        alive,
                        status,
                        name,
                        ..
                    },
                ) if !*alive && !*a0 => p0 != pid || s0 != status || n0 != name,
                (Some(prev), _) => *prev != update,
                (None, _) => true,
            };
            if changed {
                out.push(WorkspaceEvent::SideState(update.clone()));
                self.registry_emitted.insert(sid, update);
            }
        }
    }

    fn refresh_teams(&mut self) {
        let Some(home) = self.roots.claude_home.clone() else {
            return;
        };
        let paths = team_config_paths(&home);
        self.teams.retain(|p, _| paths.contains(p));
        for path in paths {
            let mtime = std::fs::metadata(&path)
                .ok()
                .and_then(|m| m.modified().ok());
            let file = self.teams.entry(path.clone()).or_insert(TeamFile {
                mtime: None,
                config: None,
            });
            if file.config.is_none() || file.mtime != mtime {
                file.config = read_team_config(&path);
                file.mtime = mtime;
            }
        }
    }

    /// Lead session id of a team (from its config).
    fn team_lead(&self, team: Option<&str>) -> Option<&str> {
        let team = team?;
        self.teams
            .values()
            .filter_map(|f| f.config.as_ref())
            .find(|c| c.name == team)
            .and_then(|c| c.lead_session_id.as_deref())
    }

    /// Agent of a team's lead: `leadSessionId` resolved through runtime aliases.
    fn team_lead_agent(&self, team: Option<&str>) -> Option<AgentId> {
        self.resolve_lead(self.team_lead(team)?)
    }

    /// `leadSessionId` → lead agent (shared rule `team_lead_agent`).
    fn resolve_lead(&self, lead_session_id: &str) -> Option<AgentId> {
        team_lead_agent(
            lead_session_id,
            |id| {
                let path = self.agents.get(id)?;
                self.tracked.get(path).map(|t| t.agent.kind)
            },
            &self.runtime_aliases,
        )
    }

    /// Report configs of teams that tracked agents refer to, or whose lead is tracked.
    fn emit_teams(&mut self, out: &mut Vec<WorkspaceEvent>) {
        let mut updates = Vec::new();
        for config in self.teams.values().filter_map(|f| f.config.as_ref()) {
            let lead_tracked = config
                .lead_session_id
                .as_deref()
                .and_then(|lead| self.resolve_lead(lead))
                .is_some_and(|lead| self.agents.contains_key(&lead));
            if !lead_tracked && !self.referenced_teams.contains(&config.name) {
                continue;
            }
            let update = SideStateUpdate::ClaudeTeam {
                team: config.name.clone(),
                lead_session_id: config.lead_session_id.clone().unwrap_or_default(),
                members: config
                    .members
                    .iter()
                    .map(|m| TeamMember {
                        name: m.name.clone(),
                        agent_type: m.agent_type.clone(),
                        model: m.model.clone(),
                        is_active: m.is_active,
                    })
                    .collect(),
            };
            if self.teams_emitted.get(&config.name) != Some(&update) {
                updates.push((config.name.clone(), update));
            }
        }
        updates.sort_by(|a, b| a.0.cmp(&b.0));
        for (team, update) in updates {
            out.push(WorkspaceEvent::SideState(update.clone()));
            self.teams_emitted.insert(team, update);
        }
    }
}

/// Entries of a directory (empty if it does not exist), sorted.
fn list_dir(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    paths.sort();
    paths
}

/// `<sessions>/YYYY/MM/DD` of a local date.
fn date_dir(sessions: &Path, day: NaiveDate) -> PathBuf {
    sessions.join(day.format("%Y/%m/%d").to_string())
}

/// Local date of a `<sessions>/YYYY/MM/DD` dir.
fn date_of_dir(sessions: &Path, dir: &Path) -> Option<NaiveDate> {
    let rel = dir
        .strip_prefix(sessions)
        .ok()?
        .to_string_lossy()
        .into_owned();
    NaiveDate::parse_from_str(&rel.replace(std::path::MAIN_SEPARATOR, "/"), "%Y/%m/%d").ok()
}

/// Codex date dirs older than yesterday that a scope starting on local date `first`
/// reaches: `first - 1` (margin for a changed UTC offset) up to the day before
/// yesterday. Rollouts live in the dir of their *creation* date, so a tree started
/// on `first` has all of its descendants in these dirs, today's or yesterday's.
/// Empty when `first` is today or yesterday (e.g. the default 2 h project window).
pub(crate) fn codex_history_range(first: NaiveDate, today: NaiveDate) -> Vec<NaiveDate> {
    let (Some(yesterday), Some(last)) = (
        today.pred_opt(),
        today.pred_opt().and_then(|d| d.pred_opt()),
    ) else {
        return Vec::new();
    };
    if first >= yesterday {
        return Vec::new();
    }
    let mut day = first.pred_opt().unwrap_or(first);
    let mut out = Vec::new();
    while day <= last {
        out.push(day);
        match day.succ_opt() {
            Some(next) => day = next,
            None => break,
        }
    }
    out
}

/// One-off lookup (names only) of the directory holding a file whose name ends with
/// `suffix` below `root`.
fn find_file_dir(root: &Path, suffix: &str) -> Option<PathBuf> {
    walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .find(|e| e.file_type().is_file() && e.file_name().to_string_lossy().ends_with(suffix))
        .and_then(|e| e.path().parent().map(Path::to_path_buf))
}
