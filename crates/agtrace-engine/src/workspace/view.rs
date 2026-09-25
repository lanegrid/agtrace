//! `WorkspaceView`: the incrementally folded agent graph + live per-agent state.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::path::PathBuf;

use agtrace_types::{
    AgentAttributeKey, AgentEvent, AgentHandle, AgentId, AgentKind, AgentMessageKind, AgentOp,
    AgentRef, AgentSpawnPayload, EventPayload, LifecycleTransition, MessageDirection,
    ParseDiagnostics, Provider, ToolCallPayload, TurnOutcome,
};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::context_seam::{ContextEvidence, ContextWindow, WindowResolver};
use super::detail::{
    AgentDetail, AgentResult, ContextPoint, DETAIL_TEXT_MAX, Instruction, InstructionKind, Said,
    cap_text,
};
use super::feed::{FeedEntry, FeedKind, FeedParty, LIFECYCLE_DEDUPE_WINDOW};
use super::input::{SideStateUpdate, TeamMember, WorkspaceEvent};
use super::parent::{RuntimeAliases, team_lead_agent, teammate_parent};
use super::ring::RingBuffer;
use super::status::{
    AgentFacts, AgentStatus, ParentContext, RegistryEntry, StatusSignals, StatusSource,
    TerminalOrigin, derive_status,
};
use super::timeline::{
    RunningTool, TIMELINE_TEXT_MAX, TimelineEntry, handle_label, one_line, timeline_item,
};

/// Feed capacity (all agents).
pub const FEED_CAPACITY: usize = 500;
/// Timeline capacity per agent.
pub const TIMELINE_CAPACITY: usize = 1000;
/// Watcher errors kept for the status bar.
pub const ERROR_CAPACITY: usize = 50;
/// Unresolved spawns / lifecycle signals kept for late discovery (oldest dropped).
pub const MAX_PENDING: usize = 1000;
const MAX_OPEN_TOOLS: usize = 32;
/// Recent spawn tool calls remembered per agent to attach their prompt to the spawn.
const MAX_SPAWN_PROMPTS: usize = 32;
const SETTLE_ROUNDS: usize = 8;

/// How the agent was spawned (from the parent's `AgentSpawn`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnInfo {
    /// The agent whose log contained the spawn.
    pub by: AgentId,
    pub kind: AgentKind,
    pub name: Option<String>,
    pub agent_type: Option<String>,
    pub requested_model: Option<String>,
    pub resolved_model: Option<String>,
    pub description: Option<String>,
    pub spawn_call_id: Option<String>,
    pub at: DateTime<Utc>,
    /// Prompt / task text of the spawn tool call (capped), when plaintext.
    pub prompt: Option<String>,
    /// The spawn tool call's prompt was encrypted (Codex).
    pub prompt_encrypted: bool,
}

impl SpawnInfo {
    fn new(by: &AgentId, s: &AgentSpawnPayload, at: DateTime<Utc>, prompt: SpawnPrompt) -> Self {
        Self {
            by: by.clone(),
            kind: s.kind,
            name: s.name.clone(),
            agent_type: s.agent_type.clone(),
            requested_model: s.requested_model.clone(),
            resolved_model: s.resolved_model.clone(),
            description: s.description.clone(),
            spawn_call_id: s.spawn_call_id.clone(),
            at,
            prompt: prompt.text,
            prompt_encrypted: prompt.encrypted,
        }
    }
}

/// Prompt of a spawn tool call, remembered until its `AgentSpawn` arrives.
#[derive(Debug, Clone, Default)]
struct SpawnPrompt {
    call: Uuid,
    text: Option<String>,
    encrypted: bool,
}

/// Live state of one agent.
#[derive(Debug, Clone)]
pub struct AgentView {
    /// Identity; `parent` / `root` / `depth` are refined by the graph.
    pub agent: AgentRef,
    /// Children in the displayed tree (linked children, plus unlinked agents shown
    /// under their root), ordered by `started_at`.
    pub children: Vec<AgentId>,
    /// Parent in the displayed tree (None = top-level row).
    pub tree_parent: Option<AgentId>,
    pub status: AgentStatus,
    pub status_source: StatusSource,
    /// Latest model: own log, else spawn / team config / meta hint.
    pub model: Option<String>,
    pub context: ContextEvidence,
    /// Recomputed whenever the evidence changes.
    pub window: Option<ContextWindow>,
    /// Most recent tool call without a result.
    pub current_tool: Option<RunningTool>,
    pub recent: RingBuffer<TimelineEntry, TIMELINE_CAPACITY>,
    /// Timestamp of the latest record in the agent's own log (None = nothing read yet).
    pub last_activity: Option<DateTime<Utc>>,
    pub diagnostics: ParseDiagnostics,
    /// Latest value per attribute key (upsert).
    pub attributes: HashMap<AgentAttributeKey, String>,
    pub spawn: Option<SpawnInfo>,
    /// False for placeholders created by events that arrived before discovery.
    pub discovered: bool,
    /// History, instructions, result and totals (overview / detail screens).
    pub detail: AgentDetail,
    signals: StatusSignals,
    spawn_prompts: VecDeque<SpawnPrompt>,
    open_tools: Vec<RunningTool>,
    own_model: Option<String>,
    hint_model: Option<String>,
    external_models: Vec<String>,
    window_dirty: bool,
}

impl AgentView {
    fn new(agent: AgentRef, discovered: bool) -> Self {
        Self {
            agent,
            children: Vec::new(),
            tree_parent: None,
            status: AgentStatus::Unknown,
            status_source: StatusSource::None,
            model: None,
            context: ContextEvidence::default(),
            window: None,
            current_tool: None,
            recent: RingBuffer::new(),
            last_activity: None,
            diagnostics: ParseDiagnostics::default(),
            attributes: HashMap::new(),
            spawn: None,
            discovered,
            detail: AgentDetail::default(),
            signals: StatusSignals::default(),
            spawn_prompts: VecDeque::new(),
            open_tools: Vec::new(),
            own_model: None,
            hint_model: None,
            external_models: Vec::new(),
            window_dirty: true,
        }
    }

    pub fn id(&self) -> &AgentId {
        &self.agent.id
    }

    /// Display label: teammate / subagent name, Codex path, title, or a short id.
    pub fn label(&self) -> String {
        let a = &self.agent;
        let short = |s: &str| s.chars().take(8).collect::<String>();
        let clip = |s: &str| one_line(s, 40);
        match a.provider {
            Provider::Codex => a
                .path
                .clone()
                .or_else(|| a.name.clone())
                .unwrap_or_else(|| short(a.id.native_session_id())),
            Provider::ClaudeCode => a
                .name
                .clone()
                .or_else(|| self.display_name().map(str::to_string))
                .or_else(|| self.spawn.as_ref().and_then(|s| s.name.clone()))
                .or_else(|| {
                    self.spawn
                        .as_ref()
                        .and_then(|s| s.description.as_deref().map(clip))
                })
                .or_else(|| {
                    self.attributes
                        .get(&AgentAttributeKey::Title)
                        .map(|t| clip(t))
                })
                .unwrap_or_else(|| {
                    short(
                        a.id.native_agent_id()
                            .unwrap_or_else(|| a.id.native_session_id()),
                    )
                }),
        }
    }

    /// The `agent-name` display name, except for teammates: theirs is the process
    /// display name inherited from the lead, not the teammate's own name.
    fn display_name(&self) -> Option<&str> {
        (self.agent.kind != AgentKind::Teammate)
            .then(|| self.attributes.get(&AgentAttributeKey::AgentName))
            .flatten()
            .map(String::as_str)
    }

    /// Transcript id this (Claude) transcript was continued in (`continued-in`).
    pub fn continued_in(&self) -> Option<&str> {
        self.attributes
            .get(&AgentAttributeKey::ContinuedIn)
            .map(String::as_str)
            .filter(|s| *s != self.agent.id.native_session_id())
    }

    /// Team of a Claude agent (header, else `team_context` attribute).
    pub fn team(&self) -> Option<&str> {
        self.agent.team.as_deref().or_else(|| {
            self.attributes
                .get(&AgentAttributeKey::TeamName)
                .map(|s| s.as_str())
        })
    }

    /// All tool calls still waiting for a result (oldest first).
    pub fn open_tools(&self) -> &[RunningTool] {
        &self.open_tools
    }

    /// Timestamp of the latest activity (not metadata) in the agent's own log.
    pub fn last_active(&self) -> Option<DateTime<Utc>> {
        self.signals.last_active
    }

    fn reset_own_state(&mut self) {
        self.recent.clear();
        self.open_tools.clear();
        self.current_tool = None;
        self.context = ContextEvidence::default();
        for m in &self.external_models {
            self.context.apply_external_model(m);
        }
        self.window_dirty = true;
        self.own_model = None;
        self.attributes.clear();
        self.last_activity = None;
        self.signals.own = None;
        self.signals.last_active = None;
        self.signals.last_write = None;
        self.signals.all_background_killed_at = None;
        self.detail.reset_own();
        self.spawn_prompts.clear();
    }

    fn add_external_model(&mut self, model: &str) {
        if !self.external_models.iter().any(|m| m == model) {
            self.external_models.push(model.to_string());
            if self.context.apply_external_model(model) {
                self.window_dirty = true;
            }
        }
        if self.hint_model.is_none() {
            self.hint_model = Some(model.to_string());
        }
    }
}

#[derive(Debug, Clone)]
struct TeamState {
    /// `leadSessionId` of the team config (possibly a runtime session id).
    lead_session_id: String,
    members: Vec<TeamMember>,
}

#[derive(Debug, Clone)]
struct SubagentMeta {
    stopped_by_user: bool,
    model: Option<String>,
}

/// What an event in agent P's log does to another agent.
#[derive(Debug, Clone)]
enum Effect {
    Link(Box<SpawnInfo>),
    Lifecycle(LifecycleTransition, Option<String>),
    Terminal(AgentStatus),
    /// The agent's result, reported in another agent's log.
    Result(Box<AgentResult>),
}

#[derive(Debug, Clone)]
struct PendingLink {
    ctx: AgentId,
    handle: AgentHandle,
    effect: Effect,
    at: DateTime<Utc>,
}

enum Resolution {
    Agent(AgentId),
    /// Might resolve once more agents are known.
    Pending,
    /// Not an agent (user, unknown).
    Never,
}

/// Agent graph + live state of a whole workspace, folded from [`WorkspaceEvent`]s.
#[derive(Debug, Clone, Default)]
pub struct WorkspaceView {
    pub agents: BTreeMap<AgentId, AgentView>,
    /// Top-level rows of the tree, ordered by `started_at` desc.
    pub roots: Vec<AgentId>,
    pub feed: RingBuffer<FeedEntry, FEED_CAPACITY>,
    pub errors: RingBuffer<String, ERROR_CAPACITY>,
    unresolved: Vec<PendingLink>,
    teams: BTreeMap<String, TeamState>,
    /// team → agent whose log spawned members of that team (fallback team lead).
    team_spawners: BTreeMap<String, AgentId>,
    /// Runtime session ids of Claude transcripts (resume / bg respawn aliases).
    runtime_aliases: RuntimeAliases,
    registry: BTreeMap<AgentId, BTreeMap<u32, RegistryEntry>>,
    subagent_meta: BTreeMap<AgentId, SubagentMeta>,
    /// Parents before children (tree pre-order).
    order: Vec<AgentId>,
    structure_dirty: bool,
}

impl WorkspaceView {
    pub fn new() -> Self {
        Self::default()
    }

    /// Fold one watcher event. `now` drives staleness; `resolver` recomputes
    /// context windows of agents whose evidence changed.
    pub fn apply(&mut self, ev: WorkspaceEvent, resolver: &dyn WindowResolver, now: DateTime<Utc>) {
        match ev {
            WorkspaceEvent::AgentDiscovered(r) | WorkspaceEvent::AgentUpdated(r) => {
                self.upsert_agent(r)
            }
            WorkspaceEvent::Events {
                agent,
                events,
                reset,
            } => self.apply_events(&agent, &events, reset),
            WorkspaceEvent::SideState(u) => self.apply_side_state(u),
            WorkspaceEvent::Diagnostics { agent, diagnostics } => {
                self.ensure_agent(&agent).diagnostics = diagnostics;
            }
            WorkspaceEvent::Error(e) => self.errors.push(e),
        }
        self.settle(now);
        self.refresh_models_and_windows(resolver);
        self.refresh_statuses(now);
    }

    /// Re-evaluate time-based (staleness) rules. Returns true if any status changed.
    pub fn tick(&mut self, now: DateTime<Utc>) -> bool {
        let before: Vec<(AgentStatus, StatusSource)> = self
            .agents
            .values()
            .map(|v| (v.status, v.status_source))
            .collect();
        self.refresh_statuses(now);
        self.agents
            .values()
            .map(|v| (v.status, v.status_source))
            .ne(before)
    }

    pub fn agent(&self, id: &AgentId) -> Option<&AgentView> {
        self.agents.get(id)
    }

    /// Tree in display order: `(agent, depth)` pre-order from `roots`.
    pub fn tree(&self) -> Vec<(&AgentId, usize)> {
        let mut out = Vec::with_capacity(self.agents.len());
        let mut stack: Vec<(&AgentId, usize)> = self.roots.iter().rev().map(|r| (r, 0)).collect();
        while let Some((id, depth)) = stack.pop() {
            out.push((id, depth));
            if let Some(v) = self.agents.get(id) {
                stack.extend(v.children.iter().rev().map(|c| (c, depth + 1)));
            }
        }
        out
    }

    /// Name from the Claude process registry, if the agent has a live entry.
    pub fn registry_name(&self, id: &AgentId) -> Option<&str> {
        self.registry
            .get(id)?
            .values()
            .filter(|e| e.alive)
            .max_by_key(|e| e.updated_at)?
            .name
            .as_deref()
    }

    /// Number of spawns / signals whose target agent is not known yet.
    pub fn pending_links(&self) -> usize {
        self.unresolved.len()
    }

    /// Sum of decode errors (invalid JSON + schema mismatches) over all agents.
    pub fn total_diagnostic_errors(&self) -> u64 {
        self.agents
            .values()
            .map(|v| v.diagnostics.error_count())
            .sum()
    }

    /// Display label of an agent, or of a feed party.
    pub fn party_label(&self, p: &FeedParty) -> String {
        match p {
            FeedParty::Agent(id) => self
                .agents
                .get(id)
                .map(|v| v.label())
                .unwrap_or_else(|| id.as_str().to_string()),
            FeedParty::User => "user".to_string(),
            FeedParty::Unresolved { label, .. } => label.clone(),
        }
    }

    // ---------------------------------------------------------------- agents

    fn upsert_agent(&mut self, r: AgentRef) {
        let id = r.id.clone();
        match self.agents.get_mut(&id) {
            Some(v) => {
                v.agent = merge_ref(&v.agent, r, v.spawn.as_ref());
                v.discovered = true;
                v.window_dirty = true;
            }
            None => {
                self.agents.insert(id, AgentView::new(r, true));
            }
        }
        self.structure_dirty = true;
    }

    fn ensure_agent(&mut self, id: &AgentId) -> &mut AgentView {
        if !self.agents.contains_key(id) {
            self.agents
                .insert(id.clone(), AgentView::new(placeholder_ref(id), false));
            self.structure_dirty = true;
        }
        self.agents.get_mut(id).expect("inserted above")
    }

    // ---------------------------------------------------------------- events

    fn apply_events(&mut self, ctx: &AgentId, events: &[AgentEvent], reset: bool) {
        let v = self.ensure_agent(ctx);
        if reset {
            v.reset_own_state();
        }
        for ev in events {
            self.apply_own_event(ctx, ev);
        }
    }

    fn apply_own_event(&mut self, ctx: &AgentId, ev: &AgentEvent) {
        let ts = ev.timestamp;
        let own_lifecycle = match &ev.payload {
            EventPayload::AgentLifecycle(l) => {
                matches!(self.resolve(ctx, &l.target), Resolution::Agent(ref t) if t == ctx)
            }
            _ => false,
        };
        let item = timeline_item(ev, &|h| self.handle_display(ctx, h));
        let instruction = self.instruction_of(ctx, ev);

        // Own state.
        let mut team_changed = false;
        let mut spawn_prompt = SpawnPrompt::default();
        {
            let v = self.agents.get_mut(ctx).expect("ensured by caller");
            v.signals.last_write = v.signals.last_write.max(Some(ts));
            v.last_activity = v.signals.last_write;
            let activity = is_activity(&ev.payload);
            if activity {
                v.signals.last_active = v.signals.last_active.max(Some(ts));
            }
            let own_before = v.signals.own;
            fold_detail(v, ev, activity, &mut spawn_prompt);
            if let Some(i) = instruction {
                v.detail.push_instruction(i);
            }
            if v.agent.started_at.is_none() {
                v.agent.started_at = Some(ts);
            }
            if v.context.apply(ev) {
                v.window_dirty = true;
            }
            match &ev.payload {
                EventPayload::TokenUsage(u) => {
                    if let Some(m) = &u.model {
                        v.own_model = Some(m.clone());
                    }
                }
                EventPayload::ModelChange(m) => v.own_model = Some(m.to.clone()),
                EventPayload::AgentAttribute(a) => {
                    team_changed = matches!(
                        a.key,
                        AgentAttributeKey::TeamName | AgentAttributeKey::ContinuedIn
                    );
                    if a.key != AgentAttributeKey::RuntimeSessionId {
                        v.attributes.insert(a.key, a.value.clone());
                    }
                    // A continued transcript is finished: its process moved on to
                    // the continuation (later activity here overrides this).
                    if a.key == AgentAttributeKey::ContinuedIn {
                        v.signals.own = Some((AgentStatus::Done, ts));
                    }
                }
                EventPayload::ToolCall(c) => {
                    if v.open_tools.len() == MAX_OPEN_TOOLS {
                        v.open_tools.remove(0);
                    }
                    v.open_tools.push(RunningTool::from_call(ev, c));
                }
                EventPayload::ToolResult(r) => {
                    v.open_tools.retain(|t| t.call_id != r.tool_call_id);
                }
                EventPayload::TurnEnd(e) => {
                    v.open_tools.clear();
                    let st = match e.outcome {
                        TurnOutcome::Failed { .. } => AgentStatus::Failed,
                        TurnOutcome::Completed | TurnOutcome::Interrupted => AgentStatus::Idle,
                    };
                    v.signals.own = Some((st, ts));
                }
                EventPayload::AgentLifecycle(l) if own_lifecycle => {
                    v.signals.own = Some((own_transition_status(l.transition), ts));
                }
                _ => {}
            }
            if activity && !own_lifecycle {
                v.signals.own = Some((AgentStatus::Running, ts));
            }
            if v.signals.own != own_before
                && let Some((st, at)) = v.signals.own
            {
                v.detail.status_history.record_own(at, st);
            }
            if let EventPayload::AgentLifecycle(l) = &ev.payload
                && own_lifecycle
                && let Some(r) = &l.reason
            {
                v.detail.end_reason = Some(one_line(r, TIMELINE_TEXT_MAX));
            }
            v.current_tool = v.open_tools.last().cloned();
            if let Some(item) = item {
                v.recent.push(TimelineEntry {
                    event_id: ev.id,
                    ts,
                    origin: ev.origin,
                    item,
                });
            }
        }
        if team_changed | self.runtime_aliases.record(ev) {
            self.structure_dirty = true;
        }

        // Effects on other agents + feed.
        match &ev.payload {
            EventPayload::AgentSpawn(s) => {
                if let AgentHandle::TeamMember { team: Some(t), .. } = &s.child
                    && !self.team_spawners.contains_key(t)
                {
                    self.team_spawners.insert(t.clone(), ctx.clone());
                    self.structure_dirty = true;
                }
                let info = SpawnInfo::new(ctx, s, ts, spawn_prompt);
                self.dispatch(ctx, &s.child, Effect::Link(Box::new(info)), ts);
                let text = s
                    .description
                    .as_deref()
                    .or(s.agent_type.as_deref())
                    .map(|d| one_line(d, TIMELINE_TEXT_MAX));
                let child = self.party(ctx, &s.child);
                self.push_feed(FeedEntry {
                    event_id: ev.id,
                    ts,
                    source: ctx.clone(),
                    from: FeedParty::Agent(ctx.clone()),
                    to: vec![child],
                    kind: FeedKind::Spawn(s.kind),
                    text,
                    encrypted: false,
                    direction: None,
                    merged: Vec::new(),
                });
            }
            EventPayload::AgentLifecycle(l) => {
                let subject = if l.transition == LifecycleTransition::AllBackgroundKilled {
                    let v = self.agents.get_mut(ctx).expect("ensured by caller");
                    v.signals.all_background_killed_at =
                        v.signals.all_background_killed_at.max(Some(ts));
                    FeedParty::Agent(ctx.clone())
                } else {
                    if !own_lifecycle {
                        self.dispatch(
                            ctx,
                            &l.target,
                            Effect::Lifecycle(l.transition, l.reason.clone()),
                            ts,
                        );
                    }
                    self.party(ctx, &l.target)
                };
                if l.transition != LifecycleTransition::Running {
                    self.push_feed(FeedEntry {
                        event_id: ev.id,
                        ts,
                        source: ctx.clone(),
                        from: subject,
                        to: Vec::new(),
                        kind: FeedKind::Lifecycle(l.transition),
                        text: l.reason.as_deref().map(|r| one_line(r, TIMELINE_TEXT_MAX)),
                        encrypted: false,
                        direction: None,
                        merged: Vec::new(),
                    });
                }
            }
            EventPayload::AgentMessage(m) => {
                if m.kind == AgentMessageKind::Handback && m.direction == MessageDirection::Incoming
                {
                    self.dispatch(ctx, &m.from, Effect::Terminal(AgentStatus::Done), ts);
                }
                if is_result_kind(&m.kind) {
                    let result = AgentResult {
                        event_id: ev.id,
                        at: ts,
                        kind: m.kind.clone(),
                        text: m.body.as_deref().map(|b| cap_text(b, DETAIL_TEXT_MAX)),
                        encrypted: m.encrypted,
                        own: m.direction == MessageDirection::Outgoing,
                    };
                    match m.direction {
                        MessageDirection::Outgoing => self
                            .agents
                            .get_mut(ctx)
                            .expect("ensured by caller")
                            .detail
                            .set_result(result),
                        MessageDirection::Incoming => {
                            self.dispatch(ctx, &m.from, Effect::Result(Box::new(result)), ts)
                        }
                    }
                }
                let from = self.party(ctx, &m.from);
                let to = m.to.iter().map(|h| self.party(ctx, h)).collect();
                self.push_feed(FeedEntry {
                    event_id: ev.id,
                    ts,
                    source: ctx.clone(),
                    from,
                    to,
                    kind: FeedKind::Message(m.kind.clone()),
                    text: m
                        .body
                        .as_deref()
                        .or(m.summary.as_deref())
                        .map(|b| one_line(b, TIMELINE_TEXT_MAX)),
                    encrypted: m.encrypted,
                    direction: Some(m.direction),
                    merged: Vec::new(),
                });
            }
            _ => {}
        }
    }

    fn apply_side_state(&mut self, u: SideStateUpdate) {
        match u {
            SideStateUpdate::ClaudeProcess {
                session_id,
                pid,
                alive,
                status,
                name,
                updated_at,
            } => {
                let entries = self
                    .registry
                    .entry(AgentId::claude_session(&session_id))
                    .or_default();
                // Ignore out-of-date reports for the same pid.
                if entries.get(&pid).is_none_or(|e| updated_at >= e.updated_at) {
                    entries.insert(
                        pid,
                        RegistryEntry {
                            alive,
                            status,
                            name,
                            updated_at,
                        },
                    );
                }
            }
            SideStateUpdate::ClaudeTeam {
                team,
                lead_session_id,
                members,
            } => {
                self.teams.insert(
                    team,
                    TeamState {
                        lead_session_id,
                        members,
                    },
                );
                self.structure_dirty = true;
            }
            SideStateUpdate::ClaudeSubagentMeta {
                agent,
                stopped_by_user,
                model,
            } => {
                self.subagent_meta.insert(
                    agent,
                    SubagentMeta {
                        stopped_by_user,
                        model,
                    },
                );
                self.structure_dirty = true;
            }
        }
    }

    // ---------------------------------------------------------------- resolution

    fn resolve(&self, ctx: &AgentId, h: &AgentHandle) -> Resolution {
        let known = |id: AgentId| {
            if self.agents.contains_key(&id) {
                Resolution::Agent(id)
            } else {
                Resolution::Pending
            }
        };
        match h {
            AgentHandle::User | AgentHandle::Unknown(_) => Resolution::Never,
            AgentHandle::Id(id) => known(id.clone()),
            AgentHandle::NativeAgentId(aid) => {
                if let Some((name, team)) = aid.split_once('@') {
                    return self.resolve_team_member(ctx, Some(team), name);
                }
                if ctx.provider() == Provider::ClaudeCode {
                    let direct = AgentId::claude_subagent(ctx.native_session_id(), aid);
                    if self.agents.contains_key(&direct) {
                        return Resolution::Agent(direct);
                    }
                }
                // Resumed sessions: the subagent may live under another session dir.
                self.agents
                    .values()
                    .filter(|v| {
                        v.agent.native_agent_id.as_deref() == Some(aid.as_str())
                            || v.agent.id.native_agent_id() == Some(aid.as_str())
                    })
                    .max_by_key(|v| v.agent.started_at)
                    .map(|v| Resolution::Agent(v.agent.id.clone()))
                    .unwrap_or(Resolution::Pending)
            }
            AgentHandle::TeamMember { team, name } => {
                self.resolve_team_member(ctx, team.as_deref(), name)
            }
            AgentHandle::Path(p) => {
                let Some(root) = self.agents.get(ctx).map(|v| v.agent.root.clone()) else {
                    return Resolution::Pending;
                };
                if p == "/root" {
                    return known(root);
                }
                self.agents
                    .values()
                    .filter(|v| v.agent.root == root && v.agent.path.as_deref() == Some(p.as_str()))
                    .max_by_key(|v| v.agent.started_at)
                    .map(|v| Resolution::Agent(v.agent.id.clone()))
                    .unwrap_or(Resolution::Pending)
            }
        }
    }

    fn resolve_team_member(&self, ctx: &AgentId, team: Option<&str>, name: &str) -> Resolution {
        let ctx_view = self.agents.get(ctx);
        let team = team.or_else(|| ctx_view.and_then(|v| v.team()));
        if name == "team-lead" {
            let Some(team) = team else {
                return Resolution::Pending;
            };
            let lead = self
                .teams
                .get(team)
                .and_then(|t| self.team_lead(t))
                .filter(|l| self.agents.contains_key(l))
                .or_else(|| self.team_spawners.get(team).cloned());
            return match lead {
                Some(l) if self.agents.contains_key(&l) => Resolution::Agent(l),
                _ => Resolution::Pending,
            };
        }
        let mut candidates: Vec<&AgentView> = self
            .agents
            .values()
            .filter(|v| v.agent.id != *ctx)
            .filter(|v| v.agent.name.as_deref() == Some(name) || v.display_name() == Some(name))
            .collect();
        match team {
            Some(t) => candidates.retain(|v| v.team() == Some(t)),
            None => {
                // Prefer members led by / spawned from the context agent.
                let led: Vec<&AgentView> = candidates
                    .iter()
                    .copied()
                    .filter(|v| v.agent.parent.as_ref() == Some(ctx))
                    .collect();
                if !led.is_empty() {
                    candidates = led;
                }
            }
        }
        candidates
            .into_iter()
            .max_by_key(|v| v.agent.started_at)
            .map(|v| Resolution::Agent(v.agent.id.clone()))
            .unwrap_or(Resolution::Pending)
    }

    /// Agent of a team's lead (`leadSessionId` may be a runtime session id).
    fn team_lead(&self, t: &TeamState) -> Option<AgentId> {
        team_lead_agent(
            &t.lead_session_id,
            |id| self.agents.get(id).map(|v| v.agent.kind),
            &self.runtime_aliases,
        )
    }

    /// Instruction carried by an event of `ctx`'s own log: a user prompt, a queued
    /// prompt absorbed mid-turn, or a message addressed to `ctx` (not a report from
    /// one of its own children, and not a result / status notification).
    fn instruction_of(&self, ctx: &AgentId, ev: &AgentEvent) -> Option<Instruction> {
        let cap = |t: &str| Some(cap_text(t, DETAIL_TEXT_MAX)).filter(|t| !t.is_empty());
        let (kind, from, text, encrypted) = match &ev.payload {
            EventPayload::User(u) => (InstructionKind::Prompt, None, cap(&u.text)?.into(), false),
            EventPayload::QueueOperation(q)
                if q.reason.as_deref().is_some_and(|r| r.contains("absorbed")) =>
            {
                (
                    InstructionKind::Queued,
                    None,
                    cap(q.content.as_deref()?)?.into(),
                    false,
                )
            }
            EventPayload::AgentMessage(m)
                if m.direction == MessageDirection::Incoming && is_instruction_kind(&m.kind) =>
            {
                if let Resolution::Agent(sender) = self.resolve(ctx, &m.from)
                    && (sender == *ctx
                        || self
                            .agents
                            .get(&sender)
                            .is_some_and(|s| s.agent.parent.as_ref() == Some(ctx)))
                {
                    return None;
                }
                let text = m.body.as_deref().or(m.summary.as_deref()).and_then(cap);
                (
                    InstructionKind::Message(m.kind.clone()),
                    Some(self.handle_display(ctx, &m.from)),
                    text,
                    m.encrypted,
                )
            }
            _ => return None,
        };
        Some(Instruction {
            event_id: ev.id,
            at: ev.timestamp,
            kind,
            from,
            text,
            encrypted,
        })
    }

    fn handle_display(&self, ctx: &AgentId, h: &AgentHandle) -> String {
        match self.resolve(ctx, h) {
            Resolution::Agent(id) => self.agents[&id].label(),
            _ => handle_label(h),
        }
    }

    fn party(&self, ctx: &AgentId, h: &AgentHandle) -> FeedParty {
        match (h, self.resolve(ctx, h)) {
            (AgentHandle::User, _) => FeedParty::User,
            (_, Resolution::Agent(id)) => FeedParty::Agent(id),
            _ => FeedParty::Unresolved {
                handle: h.clone(),
                label: handle_label(h),
            },
        }
    }

    fn dispatch(&mut self, ctx: &AgentId, h: &AgentHandle, effect: Effect, at: DateTime<Utc>) {
        match self.resolve(ctx, h) {
            Resolution::Agent(t) => {
                if t != *ctx {
                    self.apply_effect(&t, effect, at);
                }
            }
            Resolution::Pending => {
                if self.unresolved.len() >= MAX_PENDING {
                    self.unresolved.remove(0);
                }
                self.unresolved.push(PendingLink {
                    ctx: ctx.clone(),
                    handle: h.clone(),
                    effect,
                    at,
                });
            }
            Resolution::Never => {}
        }
    }

    fn apply_effect(&mut self, target: &AgentId, effect: Effect, at: DateTime<Utc>) {
        match effect {
            Effect::Link(info) => self.link(target, *info),
            Effect::Terminal(st) => {
                if let Some(v) = self.agents.get_mut(target) {
                    v.signals.set_terminal(st, at, TerminalOrigin::Event);
                    v.detail.status_history.record_report(at, st);
                }
            }
            Effect::Result(r) => {
                if let Some(v) = self.agents.get_mut(target) {
                    v.detail.set_result(*r);
                }
            }
            Effect::Lifecycle(tr, reason) => {
                let Some(v) = self.agents.get_mut(target) else {
                    return;
                };
                if let Some(st) = remote_status(tr) {
                    v.detail.status_history.record_report(at, st);
                }
                if matches!(
                    tr,
                    LifecycleTransition::Killed | LifecycleTransition::Failed
                ) && let Some(r) = reason
                {
                    v.detail.end_reason = Some(one_line(&r, TIMELINE_TEXT_MAX));
                }
                let s = &mut v.signals;
                match tr {
                    LifecycleTransition::Completed => {
                        s.set_terminal(AgentStatus::Done, at, TerminalOrigin::Event)
                    }
                    LifecycleTransition::Killed => {
                        s.set_terminal(AgentStatus::Killed, at, TerminalOrigin::Event)
                    }
                    // Teammate idle_notification(failed) is an own-state report
                    // (priority 2); a failed task-notification is terminal.
                    LifecycleTransition::Failed if v.agent.kind == AgentKind::Teammate => {
                        s.set_remote(AgentStatus::Failed, at)
                    }
                    LifecycleTransition::Failed => {
                        s.set_terminal(AgentStatus::Failed, at, TerminalOrigin::Event)
                    }
                    LifecycleTransition::Idle | LifecycleTransition::Interrupted => {
                        s.set_remote(AgentStatus::Idle, at)
                    }
                    LifecycleTransition::Running => s.set_remote(AgentStatus::Running, at),
                    LifecycleTransition::AllBackgroundKilled => {}
                }
            }
        }
    }

    /// `AgentSpawn` in `info.by` resolved to `target`: `target.parent = info.by`.
    fn link(&mut self, target: &AgentId, info: SpawnInfo) {
        if *target == info.by || self.is_ancestor(target, &info.by) {
            return;
        }
        let Some(v) = self.agents.get_mut(target) else {
            return;
        };
        let a = &mut v.agent;
        a.parent = Some(info.by.clone());
        if a.spawn_call_id.is_none() {
            a.spawn_call_id = info.spawn_call_id.clone();
        }
        if a.name.is_none() {
            a.name = info.name.clone();
        }
        if a.agent_type.is_none() {
            a.agent_type = info.agent_type.clone();
        }
        if let Some(m) = info.resolved_model.as_deref() {
            v.add_external_model(m);
        } else if v.hint_model.is_none() {
            v.hint_model = info.requested_model.clone();
        }
        v.spawn = Some(info);
        self.structure_dirty = true;
    }

    /// True if `ancestor` is on the parent chain of `id` (following `agent.parent`).
    fn is_ancestor(&self, ancestor: &AgentId, id: &AgentId) -> bool {
        let mut cur = self.agents.get(id).and_then(|v| v.agent.parent.clone());
        let mut steps = 0;
        while let Some(p) = cur {
            if p == *ancestor {
                return true;
            }
            steps += 1;
            if steps > self.agents.len() {
                return false;
            }
            cur = self.agents.get(&p).and_then(|v| v.agent.parent.clone());
        }
        false
    }

    // ---------------------------------------------------------------- settle

    fn settle(&mut self, now: DateTime<Utc>) {
        if !self.structure_dirty {
            return;
        }
        for _ in 0..SETTLE_ROUNDS {
            self.structure_dirty = false;
            self.apply_teams(now);
            self.apply_subagent_meta(now);
            self.retry_pending();
            self.rebuild_tree();
            if !self.structure_dirty {
                break;
            }
        }
        self.structure_dirty = false;
        self.resolve_feed_parties();
    }

    fn apply_teams(&mut self, now: DateTime<Utc>) {
        let teams: Vec<(String, TeamState)> = self
            .teams
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        for (team, st) in teams {
            let lead = self.team_lead(&st);
            let known: HashSet<AgentId> = self.agents.keys().cloned().collect();
            // A teammate whose team config names the lead is linked even without a spawn.
            // (A spawn seen later re-links it to the spawner: `link`.) A link to a lead
            // that is not known is retried: the lead id may resolve later (alias).
            for v in self.agents.values_mut() {
                if v.agent.kind == AgentKind::Teammate
                    && v.agent.parent.as_ref().is_none_or(|p| !known.contains(p))
                    && v.team() == Some(team.as_str())
                {
                    let spawner = v.spawn.as_ref().map(|s| &s.by);
                    let parent = teammate_parent(&v.agent.id, spawner, lead.as_ref());
                    if parent.is_some() && parent != v.agent.parent {
                        v.agent.parent = parent;
                        self.structure_dirty = true;
                    }
                }
            }
            for m in &st.members {
                let target = if m.name == "team-lead" {
                    lead.clone()
                } else {
                    self.agents
                        .values()
                        .filter(|v| {
                            v.team() == Some(team.as_str())
                                && v.agent.name.as_deref() == Some(m.name.as_str())
                        })
                        .max_by_key(|v| v.agent.started_at)
                        .map(|v| v.agent.id.clone())
                };
                let Some(v) = target.and_then(|t| self.agents.get_mut(&t)) else {
                    continue;
                };
                if let Some(model) = m.model.as_deref() {
                    v.add_external_model(model);
                }
                let config_terminal = v
                    .signals
                    .terminal
                    .is_some_and(|t| t.origin == TerminalOrigin::TeamConfig);
                match m.is_active {
                    Some(false) if !config_terminal => {
                        v.signals
                            .set_terminal(AgentStatus::Killed, now, TerminalOrigin::TeamConfig)
                    }
                    Some(true) if config_terminal => v.signals.terminal = None,
                    _ => {}
                }
            }
        }
    }

    fn apply_subagent_meta(&mut self, now: DateTime<Utc>) {
        for (id, meta) in &self.subagent_meta {
            let Some(v) = self.agents.get_mut(id) else {
                continue;
            };
            if let Some(model) = meta.model.as_deref() {
                v.add_external_model(model);
            }
            let already = v
                .signals
                .terminal
                .is_some_and(|t| t.origin == TerminalOrigin::SubagentMeta);
            if meta.stopped_by_user && !already {
                v.signals
                    .set_terminal(AgentStatus::Killed, now, TerminalOrigin::SubagentMeta);
            }
        }
    }

    fn retry_pending(&mut self) {
        let pending = std::mem::take(&mut self.unresolved);
        for p in pending {
            match self.resolve(&p.ctx, &p.handle) {
                Resolution::Agent(t) => {
                    if t != p.ctx {
                        self.apply_effect(&t, p.effect, p.at);
                    }
                }
                Resolution::Pending => self.unresolved.push(p),
                Resolution::Never => {}
            }
        }
    }

    /// Recompute tree parents, children, roots, order, and root/depth propagation.
    fn rebuild_tree(&mut self) {
        let ids: Vec<AgentId> = self.agents.keys().cloned().collect();
        // Displayed parent: linked parent if known, else the root (unlinked children
        // are shown under their root), else top-level.
        let mut tparent: BTreeMap<AgentId, Option<AgentId>> = BTreeMap::new();
        for id in &ids {
            let a = &self.agents[id].agent;
            let p = a
                .parent
                .clone()
                .filter(|p| p != id && self.agents.contains_key(p))
                .or_else(|| {
                    (a.root != *id && self.agents.contains_key(&a.root)).then(|| a.root.clone())
                })
                .or_else(|| {
                    // A continued transcript is shown under its continuation (same
                    // logical session), not as an unrelated root.
                    let next = AgentId::claude_session(self.agents[id].continued_in()?);
                    (next != *id && self.agents.contains_key(&next)).then_some(next)
                });
            tparent.insert(id.clone(), p);
        }
        // Break cycles (malformed data): an agent on a cycle becomes top-level.
        for id in &ids {
            let mut seen = HashSet::new();
            let mut cur = tparent[id].clone();
            while let Some(p) = cur {
                if p == *id {
                    tparent.insert(id.clone(), None);
                    break;
                }
                if !seen.insert(p.clone()) {
                    break;
                }
                cur = tparent.get(&p).cloned().flatten();
            }
        }

        for v in self.agents.values_mut() {
            v.children.clear();
            v.tree_parent = tparent[&v.agent.id].clone();
        }
        let mut roots = Vec::new();
        for (id, p) in &tparent {
            match p {
                Some(p) => self
                    .agents
                    .get_mut(p)
                    .expect("filtered")
                    .children
                    .push(id.clone()),
                None => roots.push(id.clone()),
            }
        }
        let started = |agents: &BTreeMap<AgentId, AgentView>, id: &AgentId| {
            agents.get(id).and_then(|v| v.agent.started_at)
        };
        let snapshot: BTreeMap<AgentId, Option<DateTime<Utc>>> = self
            .agents
            .iter()
            .map(|(k, v)| (k.clone(), v.agent.started_at))
            .collect();
        for v in self.agents.values_mut() {
            v.children
                .sort_by(|a, b| (snapshot[a], a).cmp(&(snapshot[b], b)));
        }
        // Newest first; unknown start last.
        roots.sort_by(|a, b| {
            let (sa, sb) = (started(&self.agents, a), started(&self.agents, b));
            match (sa, sb) {
                (Some(x), Some(y)) => y.cmp(&x).then_with(|| a.cmp(b)),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => a.cmp(b),
            }
        });
        self.roots = roots;
        self.order = self.tree().into_iter().map(|(id, _)| id.clone()).collect();

        // Propagate root / depth along real parent links (parents first).
        for id in self.order.clone() {
            let (parent, tp) = {
                let v = &self.agents[&id];
                (v.agent.parent.clone(), v.tree_parent.clone())
            };
            let Some(p) = parent.filter(|p| tp.as_ref() == Some(p)) else {
                continue;
            };
            let (root, depth) = {
                let pv = &self.agents[&p].agent;
                (pv.root.clone(), pv.depth + 1)
            };
            let a = &mut self.agents.get_mut(&id).expect("exists").agent;
            a.root = root;
            a.depth = depth;
        }
    }

    // ---------------------------------------------------------------- feed

    fn push_feed(&mut self, entry: FeedEntry) {
        // Replays (reset / re-read) of an event already in the feed add nothing.
        if self.feed.iter().any(|x| x.covers(&entry.event_id)) {
            return;
        }
        match self.find_duplicate(&entry, None) {
            Some(i) => self
                .feed
                .get_mut(i)
                .expect("index from find_duplicate")
                .absorb(entry),
            None => self.feed.insert_by_key(entry, |e| e.ts),
        }
    }

    /// Index of the feed entry that `e` duplicates (ignoring index `skip`):
    /// - a message: the unpaired other side (other log, opposite direction, same
    ///   parties and kind, compatible body, plausible delivery delay), oldest first;
    /// - a lifecycle report: the nearest report of the same transition of the same
    ///   agent within [`LIFECYCLE_DEDUPE_WINDOW`] with nothing addressed to the agent
    ///   in between.
    fn find_duplicate(&self, e: &FeedEntry, skip: Option<usize>) -> Option<usize> {
        let same = |a: &FeedParty, b: &FeedParty| parties_eq(&self.agents, a, b);
        let others = self
            .feed
            .iter()
            .enumerate()
            .filter(move |(i, x)| Some(*i) != skip && x.kind == e.kind);
        match e.kind {
            FeedKind::Spawn(_) => None,
            FeedKind::Message(_) => {
                if !e.merged.is_empty() {
                    return None;
                }
                others
                    .filter(|(_, x)| {
                        x.merged.is_empty()
                            && x.source != e.source
                            && x.delivery_plausible(e)
                            && same(&x.from, &e.from)
                            && x.to.len() == e.to.len()
                            && e.to.iter().all(|t| x.to.iter().any(|u| same(t, u)))
                            && x.body_compatible(e)
                    })
                    .min_by_key(|(_, x)| x.ts)
                    .map(|(i, _)| i)
            }
            FeedKind::Lifecycle(_) => others
                .filter(|(_, x)| {
                    (x.ts - e.ts).abs() <= LIFECYCLE_DEDUPE_WINDOW
                        && same(&x.from, &e.from)
                        && !self.addressed_between(&e.from, x.ts, e.ts)
                })
                .min_by_key(|(_, x)| (x.ts - e.ts).abs())
                .map(|(i, _)| i),
        }
    }

    /// True if a message or spawn addressed to `subject` lies strictly between `a` and `b`.
    fn addressed_between(&self, subject: &FeedParty, a: DateTime<Utc>, b: DateTime<Utc>) -> bool {
        let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
        self.feed.iter().any(|x| {
            x.ts > lo
                && x.ts < hi
                && matches!(x.kind, FeedKind::Message(_) | FeedKind::Spawn(_))
                && x.to.iter().any(|t| parties_eq(&self.agents, t, subject))
        })
    }

    /// Re-resolve parties that were unresolved when their entry was pushed, and
    /// merge entries that turn out to be duplicates once resolved.
    fn resolve_feed_parties(&mut self) {
        let mut updates: Vec<(usize, Option<FeedParty>, Vec<Option<FeedParty>>)> = Vec::new();
        for (i, e) in self.feed.iter().enumerate() {
            let fix = |p: &FeedParty| match p {
                FeedParty::Unresolved { handle, .. } => match self.resolve(&e.source, handle) {
                    Resolution::Agent(id) => Some(FeedParty::Agent(id)),
                    _ => None,
                },
                _ => None,
            };
            let from = fix(&e.from);
            let to: Vec<Option<FeedParty>> = e.to.iter().map(fix).collect();
            if from.is_some() || to.iter().any(Option::is_some) {
                updates.push((i, from, to));
            }
        }
        let mut changed = Vec::with_capacity(updates.len());
        for (i, from, to) in updates {
            let e = self.feed.get_mut(i).expect("index from enumerate");
            if let Some(f) = from {
                e.from = f;
            }
            for (slot, new) in e.to.iter_mut().zip(to) {
                if let Some(n) = new {
                    *slot = n;
                }
            }
            changed.push(e.event_id);
        }
        for id in changed {
            let Some(i) = self.feed.iter().position(|e| e.event_id == id) else {
                continue;
            };
            let e = self.feed.get(i).expect("index from position").clone();
            let Some(j) = self.find_duplicate(&e, Some(i)) else {
                continue;
            };
            // Keep the older entry (stable feed order), fold the newer one into it.
            let older_is_j = self.feed.get(j).is_some_and(|x| x.ts <= e.ts);
            let (keep, drop) = if older_is_j { (j, i) } else { (i, j) };
            let dropped = self.feed.remove(drop).expect("index in range");
            let keep = if drop < keep { keep - 1 } else { keep };
            self.feed
                .get_mut(keep)
                .expect("index in range")
                .absorb(dropped);
        }
    }

    // ---------------------------------------------------------------- refresh

    fn refresh_models_and_windows(&mut self, resolver: &dyn WindowResolver) {
        for v in self.agents.values_mut() {
            v.model = v.own_model.clone().or_else(|| v.hint_model.clone());
            if v.window_dirty {
                v.window = resolver.resolve(&v.agent, &v.context);
                v.window_dirty = false;
            }
        }
    }

    fn refresh_statuses(&mut self, now: DateTime<Utc>) {
        let empty = BTreeMap::new();
        for id in self.order.clone() {
            let parent = match self.agents[&id].tree_parent.as_ref() {
                Some(p) => {
                    let pv = &self.agents[p];
                    ParentContext {
                        status: Some(pv.status),
                        all_background_killed_at: pv.signals.all_background_killed_at,
                    }
                }
                None => ParentContext {
                    status: None,
                    all_background_killed_at: None,
                },
            };
            let registry = self.registry.get(&id).unwrap_or(&empty);
            let v = self.agents.get_mut(&id).expect("from order");
            let facts = AgentFacts {
                provider: v.agent.provider,
                kind: v.agent.kind,
                is_root: v.agent.kind == AgentKind::Main,
            };
            let (status, source) = derive_status(facts, &v.signals, registry, parent, now);
            v.status = status;
            v.status_source = source;
        }
    }
}

/// Events that mean the agent is doing something (as opposed to metadata / turn end).
fn is_activity(p: &EventPayload) -> bool {
    !matches!(
        p,
        EventPayload::AgentAttribute(_)
            | EventPayload::ContextWindowHint(_)
            | EventPayload::ModelChange(_)
            | EventPayload::Notification(_)
            | EventPayload::QueueOperation(_)
            | EventPayload::TurnEnd(_)
    )
}

/// Fold one own-log event into the agent's detail state (activity, context series,
/// totals, last text, spawn prompts). `spawn_prompt` receives the prompt of the
/// spawn tool call an `AgentSpawn` refers to.
fn fold_detail(v: &mut AgentView, ev: &AgentEvent, activity: bool, spawn_prompt: &mut SpawnPrompt) {
    let ts = ev.timestamp;
    let d = &mut v.detail;
    match &ev.payload {
        EventPayload::TokenUsage(u) => {
            d.totals.add_usage(u);
            d.push_context(ContextPoint {
                at: ts,
                tokens: Some(u.context_tokens()),
                compaction: false,
            });
        }
        EventPayload::Compaction(c) => {
            d.compactions += 1;
            d.activity.record(ts, true);
            d.push_context(ContextPoint {
                at: ts,
                tokens: c.post_tokens,
                compaction: true,
            });
        }
        EventPayload::TurnEnd(e) => {
            d.totals.turns += 1;
            if let TurnOutcome::Failed { error: Some(err) } = &e.outcome {
                d.end_reason = Some(one_line(err, TIMELINE_TEXT_MAX));
            }
        }
        EventPayload::ToolCall(c) => {
            d.totals.tool_calls += 1;
            if let ToolCallPayload::Agent { arguments, .. } = c {
                match arguments.op {
                    AgentOp::Spawn => {
                        if v.spawn_prompts.len() == MAX_SPAWN_PROMPTS {
                            v.spawn_prompts.pop_front();
                        }
                        v.spawn_prompts.push_back(SpawnPrompt {
                            call: ev.id,
                            text: arguments
                                .message_preview
                                .as_deref()
                                .map(|t| cap_text(t, DETAIL_TEXT_MAX)),
                            encrypted: arguments.encrypted,
                        });
                    }
                    AgentOp::Handback => {
                        if let Some(text) = arguments.message_preview.as_deref() {
                            d.set_result(AgentResult {
                                event_id: ev.id,
                                at: ts,
                                kind: AgentMessageKind::Handback,
                                text: Some(cap_text(text, DETAIL_TEXT_MAX)),
                                encrypted: false,
                                own: true,
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
        EventPayload::AgentSpawn(s) => {
            if let Some(call) = s.tool_call_id
                && let Some(i) = v.spawn_prompts.iter().position(|p| p.call == call)
            {
                *spawn_prompt = v.spawn_prompts.remove(i).unwrap_or_default();
            }
        }
        EventPayload::Message(m) if !m.text.trim().is_empty() => {
            d.last_message = Some(Said {
                at: ts,
                text: cap_text(&m.text, DETAIL_TEXT_MAX),
            });
        }
        EventPayload::Reasoning(r) if !r.text.trim().is_empty() => {
            d.last_reasoning = Some(Said {
                at: ts,
                text: cap_text(&r.text, DETAIL_TEXT_MAX),
            });
        }
        _ => {}
    }
    if activity
        && !matches!(
            ev.payload,
            EventPayload::TokenUsage(_) | EventPayload::Compaction(_)
        )
    {
        v.detail.activity.record(ts, false);
    }
}

/// Message kinds that ask the recipient to do something.
fn is_instruction_kind(k: &AgentMessageKind) -> bool {
    matches!(
        k,
        AgentMessageKind::NewTask
            | AgentMessageKind::Message
            | AgentMessageKind::Peer
            | AgentMessageKind::Interrupt
            | AgentMessageKind::Other(_)
    )
}

/// Message kinds that carry the sender's result.
fn is_result_kind(k: &AgentMessageKind) -> bool {
    matches!(
        k,
        AgentMessageKind::FinalAnswer
            | AgentMessageKind::Handback
            | AgentMessageKind::TaskNotification
    )
}

/// Status a parent-side lifecycle report puts the target in (status history only;
/// the effective status is derived by [`derive_status`]).
fn remote_status(tr: LifecycleTransition) -> Option<AgentStatus> {
    Some(match tr {
        LifecycleTransition::Completed => AgentStatus::Done,
        LifecycleTransition::Killed => AgentStatus::Killed,
        LifecycleTransition::Failed => AgentStatus::Failed,
        LifecycleTransition::Idle | LifecycleTransition::Interrupted => AgentStatus::Idle,
        LifecycleTransition::Running => AgentStatus::Running,
        LifecycleTransition::AllBackgroundKilled => return None,
    })
}

fn own_transition_status(tr: LifecycleTransition) -> AgentStatus {
    match tr {
        LifecycleTransition::Running => AgentStatus::Running,
        LifecycleTransition::Idle | LifecycleTransition::Interrupted => AgentStatus::Idle,
        LifecycleTransition::Completed => AgentStatus::Done,
        LifecycleTransition::Failed => AgentStatus::Failed,
        LifecycleTransition::Killed | LifecycleTransition::AllBackgroundKilled => {
            AgentStatus::Killed
        }
    }
}

fn parties_eq(agents: &BTreeMap<AgentId, AgentView>, a: &FeedParty, b: &FeedParty) -> bool {
    let label = |p: &FeedParty| match p {
        FeedParty::Agent(id) => agents
            .get(id)
            .map(|v| v.label())
            .unwrap_or_else(|| id.as_str().to_string()),
        FeedParty::User => "user".to_string(),
        FeedParty::Unresolved { label, .. } => label.clone(),
    };
    match (a, b) {
        (FeedParty::Agent(x), FeedParty::Agent(y)) => x == y,
        (FeedParty::User, FeedParty::User) => true,
        _ => label(a) == label(b),
    }
}

/// Minimal identity for an agent whose events arrived before its header.
fn placeholder_ref(id: &AgentId) -> AgentRef {
    let mut r = AgentRef::root(id.clone(), id.native_session_id(), PathBuf::new());
    if let Some(aid) = id.native_agent_id() {
        let session = AgentId::claude_session(id.native_session_id());
        r.kind = AgentKind::Subagent;
        r.parent = Some(session.clone());
        r.root = session;
        r.native_agent_id = Some(aid.to_string());
        r.depth = 1;
    }
    r
}

/// Merge a (re-)read header into the current identity: new header data wins, but
/// graph-derived links and fields the new header lacks are kept.
fn merge_ref(old: &AgentRef, mut new: AgentRef, spawn: Option<&SpawnInfo>) -> AgentRef {
    if new.parent.is_none() {
        new.parent = old.parent.clone();
    }
    if let Some(s) = spawn
        && s.by != new.id
    {
        new.parent = Some(s.by.clone());
    }
    if new.root == new.id && old.root != old.id {
        new.root = old.root.clone();
    }
    if new.depth == 0 {
        new.depth = old.depth;
    }
    macro_rules! keep {
        ($($f:ident),*) => {$(
            if new.$f.is_none() {
                new.$f = old.$f.clone();
            }
        )*};
    }
    keep!(
        native_agent_id,
        name,
        path,
        agent_type,
        team,
        spawn_call_id,
        cwd,
        started_at
    );
    if new.file.as_os_str().is_empty() {
        new.file = old.file.clone();
    }
    new
}
