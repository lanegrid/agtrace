//! Presenter of the multi-agent watch TUI (design §6.3).
//!
//! [`build_screen`] is pure: `(&WorkspaceView, &UiState, now) -> WatchScreenVm`.
//! It decides what is shown (requirements "selected" items only):
//! - tree: roster with status, kind badge, context %;
//! - focus: current activity + timeline of the selected agent (tool calls, Codex
//!   sub-actions, messages, spawns, lifecycle, compaction, model change, turn end /
//!   interrupt, queued prompts absorbed mid-turn, user prompts, assistant text);
//! - feed: inter-agent messages, spawns and lifecycle changes (encrypted bodies are
//!   never shown, only type and route).

mod detail;
mod overview;
mod sessions;

pub use detail::SPARK_WIDTH;

use std::collections::{HashMap, HashSet};
use std::time::Instant;

use agtrace_sdk::types::{
    AgentId, AgentKind, AgentMessageKind, AgentProvider, CompactionTrigger, LifecycleTransition,
    MessageDirection, SubActionStatus, TurnOutcome,
};
use agtrace_sdk::workspace::{
    AgentStatus, AgentView, FeedEntry, FeedKind, FeedParty, Session, SessionFold, SessionState,
    TimelineEntry, TimelineItem, WorkspaceView,
};
use chrono::{DateTime, FixedOffset, Utc};

use crate::presentation::view_models::watch::{
    ActivityVm, AgentRowVm, AgentTimelineVm, ConsoleVm, CtxVm, DONE_FOLD_MIN, FeedFilter,
    FeedRowKind, FeedRowVm, FocusVm, KeyedRow, RowKind, Screen, StatusBarVm, StatusVm,
    TimelineRowVm, UiState, WatchScreenVm,
};

/// Build the whole screen from the current workspace snapshot.
///
/// Toast expiry is judged against the monotonic clock (the workspace clock `now`
/// may be frozen for fixtures).
pub fn build_screen(view: &WorkspaceView, ui: &UiState, now: DateTime<Utc>) -> WatchScreenVm {
    let filter = ui.filter.to_lowercase();
    let all_sessions = view.sessions(now);
    let focus = focus_root(view, ui);
    // Session order (live first), narrowed to the focused session; the compact
    // overview leaves the older sessions to the sessions screen.
    // (Only when there is something newer: an old `--session` or a quiet project
    // still shows its sessions.)
    let compact = focus.is_none()
        && !ui.show_older
        && rows_screen(ui) == Screen::Overview
        && all_sessions
            .iter()
            .any(|s| s.has_transcript && s.state != SessionState::Older);
    let roots: Vec<AgentId> = all_sessions
        .iter()
        .filter(|s| s.has_transcript && focus.as_ref().is_none_or(|f| s.root == *f))
        .filter(|s| !compact || s.state != SessionState::Older)
        .map(|s| s.root.clone())
        .collect();
    let fold = done_fold(ui);
    let visible = visibility(view, &roots, fold, &filter);
    let selected = effective_selection(view, ui, &roots, &visible);

    let mut tree = Vec::new();
    let mut seen = HashSet::new();
    let shown_roots: Vec<&AgentId> = roots.iter().filter(|r| visible.contains(*r)).collect();
    let n = shown_roots.len();
    for (i, root) in shown_roots.into_iter().enumerate() {
        push_rows(
            view,
            ui,
            &visible,
            selected.as_ref(),
            root,
            0,
            &mut Vec::new(),
            i + 1 == n,
            &mut tree,
            &mut seen,
        );
    }

    // Root rows are named like their session.
    for row in tree.iter_mut().filter(|r| r.depth == 0) {
        if let Some(s) = all_sessions.iter().find(|s| s.root.as_str() == row.id) {
            row.label = s.name.clone();
        }
    }

    let members: HashSet<&AgentId> = roots.iter().flat_map(|r| view.session_agents(r)).collect();
    let focus_view = build_focus(view, ui, selected.as_ref(), now);
    let feed = build_feed(
        view,
        ui,
        selected.as_ref(),
        focus.is_some().then_some(&members),
    );
    let focus_name = focus.as_ref().map(|f| {
        all_sessions
            .iter()
            .find(|s| s.root == *f)
            .map(|s| s.name.clone())
            .unwrap_or_else(|| f.native_session_id().chars().take(8).collect())
    });
    let mut status = build_status(view, ui, &members, &visible, &all_sessions, focus_name);
    if ui.screen == Screen::Sessions {
        status.folded = 0;
    }
    if !filter.is_empty() {
        status.matches = members
            .iter()
            .filter_map(|id| view.agent(id))
            .filter(|a| visible.contains(a.id()) && matches_filter(view, a, &filter))
            .count();
    }
    let sessions = sessions::build_sessions(view, ui, &all_sessions, focus.as_ref(), now);
    let overview = (ui.screen == Screen::Overview).then(|| {
        let mut ov = overview::build_overview(view, ui, &tree, &all_sessions, &visible, now);
        if compact {
            ov.older_hidden = sessions.older;
        }
        ov
    });
    // The detail screen keeps the agent it was opened on (auto-select or tree
    // changes do not switch it); it falls back to the selection.
    let detail = (ui.screen == Screen::Detail)
        .then(|| {
            ui.detail_agent
                .as_deref()
                .and_then(AgentId::parse)
                .and_then(|id| view.agent(&id))
                .or_else(|| selected.as_ref().and_then(|id| view.agent(id)))
                .map(|a| detail::build_detail(view, ui, a, now))
        })
        .flatten();

    WatchScreenVm {
        screen: ui.screen,
        sessions,
        overview,
        detail,
        tree,
        focus: focus_view,
        feed,
        status,
        focus_pane: ui.focus,
        timeline_scroll: ui.timeline_scroll,
        feed_scroll: ui.feed_scroll,
        show_help: ui.show_help,
        toast: ui
            .toast
            .as_ref()
            .filter(|t| t.is_live(Instant::now()))
            .map(|t| t.text.clone()),
    }
}

/// Root of the focused session: the focus id, or the session it has since been
/// folded into (a stub that turned out to belong to another session).
fn focus_root(view: &WorkspaceView, ui: &UiState) -> Option<AgentId> {
    let id = ui.session_focus.as_deref().and_then(AgentId::parse)?;
    Some(match view.agent(&id) {
        Some(_) => view.session_root(&id).clone(),
        None => id,
    })
}

/// Build the console (line printer) snapshot: the screen plus every agent's
/// timeline and the workspace feed, keyed by event id so the printer emits each
/// row once (design §6.3, `watch --mode console`).
pub fn build_console(view: &WorkspaceView, ui: &UiState, now: DateTime<Utc>) -> ConsoleVm {
    let ui = UiState {
        screen: Screen::Agents,
        feed_filter: FeedFilter::All,
        show_done: true,
        session_focus: None,
        collapsed: Default::default(),
        filter: String::new(),
        ..ui.clone()
    };
    let mut screen = build_screen(view, &ui, now);
    // The console prefixes timeline rows with agent labels (`[/root]`): root rows
    // keep theirs instead of the session name.
    for row in screen.tree.iter_mut().filter(|r| r.depth == 0) {
        if let Some(a) = AgentId::parse(&row.id).and_then(|id| view.agent(&id)) {
            row.label = tree_label(view, a);
        }
    }
    let timelines = screen
        .tree
        .iter()
        .filter_map(|row| {
            let id = AgentId::parse(&row.id)?;
            let a = view.agent(&id)?;
            let rows = a
                .recent
                .iter()
                .filter_map(|e| {
                    timeline_row(e, ui.utc_offset).map(|row| KeyedRow {
                        key: e.event_id.to_string(),
                        row,
                    })
                })
                .collect();
            Some(AgentTimelineVm {
                agent_id: row.id.clone(),
                label: a.label(),
                rows,
            })
        })
        .collect();
    // FeedFilter::All keeps `screen.feed` aligned with `view.feed`.
    let feed_keys = view.feed.iter().map(|e| e.event_id.to_string()).collect();
    ConsoleVm {
        screen,
        timelines,
        feed_keys,
    }
}

// ============================================================================
// Tree
// ============================================================================

/// Agents that the done fold hides: finished ones (failed ones stay visible),
/// and other transcripts of the session (earlier, `/clear` stubs) unless running.
fn is_foldable(a: &AgentView) -> bool {
    matches!(a.status, AgentStatus::Done | AgentStatus::Killed)
        || (a.session_fold.is_some() && a.status != AgentStatus::Running)
}

/// How finished agents are folded on the current screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DoneFold {
    /// `d`: everything shown.
    Off,
    /// Overview: every finished agent below a session root.
    All,
    /// Agents tree: a parent's finished children when it has more than `n`.
    Crowded(usize),
}

/// Screen whose rows `vm.tree` holds (the detail steps through its origin's).
fn rows_screen(ui: &UiState) -> Screen {
    match ui.screen {
        Screen::Detail => ui.detail_return,
        s => s,
    }
}

fn done_fold(ui: &UiState) -> DoneFold {
    if ui.show_done {
        return DoneFold::Off;
    }
    match rows_screen(ui) {
        Screen::Agents => DoneFold::Crowded(DONE_FOLD_MIN),
        Screen::Overview | Screen::Sessions | Screen::Detail => DoneFold::All,
    }
}

/// The agent's labels or id contain `needle` (lowercase), ignoring case.
fn matches_filter(view: &WorkspaceView, a: &AgentView, needle: &str) -> bool {
    [
        a.label(),
        tree_label(view, a),
        a.id().as_str().to_string(),
        a.agent.agent_type.clone().unwrap_or_default(),
    ]
    .iter()
    .any(|s| s.to_lowercase().contains(needle))
}

/// Agents shown below `roots`: all but the finished ones `fold` hides (kept when a
/// descendant is shown); with a `/` filter (lowercase, non-empty), only the
/// matching agents and their ancestors (folds ignored: finished agents are found).
fn visibility(
    view: &WorkspaceView,
    roots: &[AgentId],
    fold: DoneFold,
    filter: &str,
) -> HashSet<AgentId> {
    /// Returns whether `id` is shown.
    fn visit(
        view: &WorkspaceView,
        id: &AgentId,
        depth: usize,
        fold: DoneFold,
        filter: &str,
        out: &mut HashSet<AgentId>,
        seen: &mut HashSet<AgentId>,
    ) -> bool {
        if !seen.insert(id.clone()) {
            return false;
        }
        let Some(a) = view.agent(id) else {
            return false;
        };
        let shown_children: Vec<bool> = a
            .children
            .iter()
            .map(|c| visit(view, c, depth + 1, fold, filter, out, seen))
            .collect();
        let mut any_child = shown_children.iter().any(|x| *x);
        if !filter.is_empty() {
            let shown = any_child || matches_filter(view, a, filter);
            if shown {
                out.insert(id.clone());
            }
            return shown;
        }
        // Crowded fold: a parent with few finished children keeps them.
        if let DoneFold::Crowded(n) = fold {
            let folded: Vec<&AgentId> = a
                .children
                .iter()
                .zip(&shown_children)
                .filter(|(c, shown)| !**shown && out_candidate(view, c))
                .map(|(c, _)| c)
                .collect();
            if !folded.is_empty() && folded.len() <= n {
                for c in folded {
                    restore(view, c, out);
                }
                any_child = true;
            }
        }
        let shown = depth == 0
            || any_child
            || match fold {
                DoneFold::Off => true,
                DoneFold::All | DoneFold::Crowded(_) => !is_foldable(a),
            };
        if shown {
            out.insert(id.clone());
        }
        shown
    }
    /// A child hidden by the fold (finished, no shown descendant).
    fn out_candidate(view: &WorkspaceView, id: &AgentId) -> bool {
        view.agent(id).is_some_and(is_foldable)
    }
    /// Show a folded subtree again (everything below it is finished).
    fn restore(view: &WorkspaceView, id: &AgentId, out: &mut HashSet<AgentId>) {
        let mut stack = vec![id];
        while let Some(cur) = stack.pop() {
            if !out.insert(cur.clone()) {
                continue;
            }
            if let Some(a) = view.agent(cur) {
                stack.extend(a.children.iter());
            }
        }
    }
    let mut out = HashSet::new();
    let mut seen = HashSet::new();
    for r in roots {
        visit(view, r, 0, fold, filter, &mut out, &mut seen);
    }
    out
}

/// Parent chain in the displayed tree (nearest first).
fn tree_ancestors<'a>(view: &'a WorkspaceView, id: &AgentId) -> Vec<&'a AgentId> {
    let mut out = Vec::new();
    let mut cur = view.agent(id).and_then(|a| a.tree_parent.as_ref());
    while let Some(p) = cur {
        if out.contains(&p) || out.len() > 64 {
            break;
        }
        out.push(p);
        cur = view.agent(p).and_then(|a| a.tree_parent.as_ref());
    }
    out
}

fn is_row_shown(
    view: &WorkspaceView,
    ui: &UiState,
    visible: &HashSet<AgentId>,
    id: &AgentId,
) -> bool {
    visible.contains(id)
        && tree_ancestors(view, id)
            .iter()
            .all(|a| !is_folded(ui, a.as_str()))
}

/// Folds are ignored while a `/` filter is active (matches must stay visible).
fn is_folded(ui: &UiState, id: &str) -> bool {
    ui.filter.is_empty() && ui.collapsed.contains(id)
}

/// Selected agent: the most recently active one (auto), else the remembered one
/// (or its nearest shown ancestor when collapsed / hidden), else the first row.
fn effective_selection(
    view: &WorkspaceView,
    ui: &UiState,
    roots: &[AgentId],
    visible: &HashSet<AgentId>,
) -> Option<AgentId> {
    let shown = |id: &AgentId| is_row_shown(view, ui, visible, id);
    if ui.auto_select {
        let best = view
            .agents
            .values()
            .filter(|a| shown(a.id()))
            .filter_map(|a| a.last_activity.map(|t| (t, a.id())))
            .max_by(|x, y| x.0.cmp(&y.0).then_with(|| y.1.cmp(x.1)));
        if let Some((_, id)) = best {
            return Some(id.clone());
        }
    }
    if let Some(sel) = ui.selected.as_deref().and_then(AgentId::parse)
        && let Some((id, _)) = view.agents.get_key_value(&sel)
    {
        if shown(id) {
            return Some(id.clone());
        }
        // Filtered out: jump to the first match rather than an unrelated ancestor.
        if !ui.filter.is_empty()
            && let Some(m) = first_match(view, roots, visible, &ui.filter.to_lowercase())
        {
            return Some(m);
        }
        if let Some(a) = tree_ancestors(view, id).into_iter().find(|a| shown(a)) {
            return Some(a.clone());
        }
    }
    if !ui.filter.is_empty()
        && let Some(m) = first_match(view, roots, visible, &ui.filter.to_lowercase())
    {
        return Some(m);
    }
    roots.iter().find(|r| shown(r)).cloned()
}

/// First agent matching the filter, in tree order.
fn first_match(
    view: &WorkspaceView,
    roots: &[AgentId],
    visible: &HashSet<AgentId>,
    needle: &str,
) -> Option<AgentId> {
    let mut stack: Vec<&AgentId> = roots.iter().rev().collect();
    let mut seen = HashSet::new();
    while let Some(id) = stack.pop() {
        if !visible.contains(id) || !seen.insert(id) {
            continue;
        }
        let Some(a) = view.agent(id) else { continue };
        if matches_filter(view, a, needle) {
            return Some(id.clone());
        }
        stack.extend(a.children.iter().rev());
    }
    None
}

#[allow(clippy::too_many_arguments)]
fn push_rows(
    view: &WorkspaceView,
    ui: &UiState,
    visible: &HashSet<AgentId>,
    selected: Option<&AgentId>,
    id: &AgentId,
    depth: u16,
    guides: &mut Vec<bool>,
    is_last: bool,
    out: &mut Vec<AgentRowVm>,
    seen: &mut HashSet<AgentId>,
) {
    if !seen.insert(id.clone()) {
        return;
    }
    let Some(a) = view.agent(id) else {
        return;
    };
    let children: Vec<&AgentId> = a.children.iter().filter(|c| visible.contains(*c)).collect();
    let collapsed = is_folded(ui, id.as_str()) && !children.is_empty();
    let hidden_descendants = if collapsed {
        count_descendants(view, visible, id)
    } else {
        0
    };
    let folded_done = if ui.filter.is_empty() && !collapsed {
        a.children.iter().filter(|c| !visible.contains(*c)).count()
    } else {
        0
    };
    out.push(AgentRowVm {
        id: id.as_str().to_string(),
        depth,
        guides: guides.clone(),
        is_last_sibling: is_last,
        label: tree_label(view, a),
        provider: provider_name(a.agent.provider).to_string(),
        badge: badge(a.agent.kind),
        status: status_vm(a.status),
        ctx_pct: ctx(a).map(|c| c.pct),
        selected: selected == Some(id),
        collapsed,
        has_children: !children.is_empty(),
        hidden_descendants,
        folded_done,
        busy_tool: a.current_tool.is_some(),
    });
    if collapsed {
        return;
    }
    // Guides are recorded for levels below the roots only (roots have no connector).
    if depth > 0 {
        guides.push(!is_last);
    }
    let n = children.len();
    for (i, c) in children.into_iter().enumerate() {
        push_rows(
            view,
            ui,
            visible,
            selected,
            c,
            depth + 1,
            guides,
            i + 1 == n,
            out,
            seen,
        );
    }
    if depth > 0 {
        guides.pop();
    }
}

/// Tree label: Codex children show their path relative to the parent row
/// (`/root/judge/x` under `/root/judge` ⇒ `x`); a Claude transcript folded into
/// its session is marked as the earlier transcript or by its command (`/clear`);
/// everything else uses the agent label.
fn tree_label(view: &WorkspaceView, a: &AgentView) -> String {
    // Folded transcripts usually carry the session's name: what they are comes
    // first, so it survives clipping in narrow panes.
    match a.session_fold {
        Some(SessionFold::Continued | SessionFold::RuntimeAlias) => {
            return format!("earlier transcript · {}", a.label());
        }
        Some(SessionFold::Stub) => {
            let what = a
                .first_command()
                .unwrap_or_else(|| "no conversation".to_string());
            return format!("{what} · {}", a.label());
        }
        None => {}
    }
    let parent_path = a
        .tree_parent
        .as_ref()
        .and_then(|p| view.agent(p))
        .and_then(|p| p.agent.path.as_deref());
    if let (Some(path), Some(pp)) = (a.agent.path.as_deref(), parent_path)
        && let Some(rel) = path.strip_prefix(pp).and_then(|r| r.strip_prefix('/'))
        && !rel.is_empty()
    {
        return rel.to_string();
    }
    a.label()
}

fn count_descendants(view: &WorkspaceView, visible: &HashSet<AgentId>, id: &AgentId) -> usize {
    let mut n = 0;
    let mut stack: Vec<&AgentId> = vec![id];
    let mut seen = HashSet::new();
    while let Some(cur) = stack.pop() {
        if !seen.insert(cur) {
            continue;
        }
        if let Some(a) = view.agent(cur) {
            for c in a.children.iter().filter(|c| visible.contains(*c)) {
                n += 1;
                stack.push(c);
            }
        }
    }
    n
}

fn provider_name(p: AgentProvider) -> &'static str {
    match p {
        AgentProvider::ClaudeCode => "claude_code",
        AgentProvider::Codex => "codex",
    }
}

fn badge(kind: AgentKind) -> Option<char> {
    match kind {
        AgentKind::Teammate => Some('T'),
        AgentKind::Subagent => Some('S'),
        AgentKind::Fork => Some('F'),
        AgentKind::Main | AgentKind::CodexThread => None,
    }
}

fn kind_name(kind: AgentKind) -> &'static str {
    match kind {
        AgentKind::Main => "main",
        AgentKind::Teammate => "teammate",
        AgentKind::Subagent => "subagent",
        AgentKind::Fork => "fork",
        AgentKind::CodexThread => "thread",
    }
}

fn status_vm(s: AgentStatus) -> StatusVm {
    match s {
        AgentStatus::Running => StatusVm::Running,
        AgentStatus::Idle => StatusVm::Idle,
        AgentStatus::Done => StatusVm::Done,
        AgentStatus::Killed => StatusVm::Killed,
        AgentStatus::Failed => StatusVm::Failed,
        AgentStatus::Unknown => StatusVm::Unknown,
    }
}

/// Context occupancy; None when the window is unknown or no usage was seen.
fn ctx(a: &AgentView) -> Option<CtxVm> {
    let w = a.window.as_ref()?;
    let used = a.context.last_context_tokens;
    if used == 0 || w.tokens == 0 {
        return None;
    }
    let pct = (w.usage_ratio(used) * 100.0).round().clamp(0.0, 999.0) as u16;
    Some(CtxVm {
        pct,
        used_tokens: used,
        window_tokens: w.tokens,
        provenance: w.provenance().to_string(),
    })
}

// ============================================================================
// Focus pane
// ============================================================================

fn build_focus(
    view: &WorkspaceView,
    ui: &UiState,
    selected: Option<&AgentId>,
    now: DateTime<Utc>,
) -> FocusVm {
    let Some(a) = selected.and_then(|id| view.agent(id)) else {
        return FocusVm {
            agent_id: None,
            title: "waiting for agents…".to_string(),
            provider: String::new(),
            kind: String::new(),
            team: None,
            status: StatusVm::Unknown,
            model: None,
            ctx: None,
            activity: None,
            rows: Vec::new(),
            follow: ui.timeline_scroll.is_follow(),
        };
    };
    let offset = ui.utc_offset;
    FocusVm {
        agent_id: Some(a.id().as_str().to_string()),
        title: a.label(),
        provider: provider_name(a.agent.provider).to_string(),
        kind: kind_name(a.agent.kind).to_string(),
        team: a.team().map(str::to_string),
        status: status_vm(a.status),
        model: a.model.clone(),
        ctx: ctx(a),
        activity: activity(a, now),
        rows: a
            .recent
            .iter()
            .filter_map(|e| timeline_row(e, offset))
            .collect(),
        follow: ui.timeline_scroll.is_follow(),
    }
}

fn activity(a: &AgentView, now: DateTime<Utc>) -> Option<ActivityVm> {
    if let Some(t) = &a.current_tool {
        return Some(ActivityVm::Tool {
            name: t.name.clone(),
            summary: t.summary.clone(),
            elapsed_secs: (now - t.since).num_seconds().max(0),
            more: a.open_tools().len().saturating_sub(1),
        });
    }
    a.recent.iter().rev().find_map(|e| match &e.item {
        TimelineItem::TurnEnd { outcome, .. } => Some(ActivityVm::TurnEnded {
            outcome: outcome_name(outcome).to_string(),
            ago_secs: (now - e.ts).num_seconds().max(0),
        }),
        _ => None,
    })
}

fn outcome_name(o: &TurnOutcome) -> &'static str {
    match o {
        TurnOutcome::Completed => "completed",
        TurnOutcome::Interrupted => "interrupted",
        TurnOutcome::Failed { .. } => "failed",
    }
}

fn hhmm(ts: DateTime<Utc>, offset: FixedOffset) -> String {
    ts.with_timezone(&offset).format("%H:%M").to_string()
}

/// Compact token count: `974k`, `1.0M`, `850`.
pub fn tokens(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{}k", (n as f64 / 1_000.0).round() as u64)
    } else {
        n.to_string()
    }
}

fn duration_label(ms: u64) -> String {
    let s = ms / 1000;
    if s >= 3600 {
        format!("{}h{:02}m", s / 3600, (s % 3600) / 60)
    } else if s >= 60 {
        format!("{}m{:02}s", s / 60, s % 60)
    } else {
        format!("{:.1}s", ms as f64 / 1000.0)
    }
}

fn message_tag(kind: &AgentMessageKind) -> String {
    match kind {
        AgentMessageKind::Message => "MESSAGE".to_string(),
        AgentMessageKind::NewTask => "NEW_TASK".to_string(),
        AgentMessageKind::FinalAnswer => "FINAL_ANSWER".to_string(),
        AgentMessageKind::Handback => "HANDBACK".to_string(),
        AgentMessageKind::TaskNotification => "TASK_NOTIFY".to_string(),
        AgentMessageKind::IdleNotification => "IDLE".to_string(),
        AgentMessageKind::Interrupt => "INTERRUPT".to_string(),
        AgentMessageKind::Peer => "PEER".to_string(),
        AgentMessageKind::Other(s) => s.to_uppercase(),
    }
}

fn transition_name(t: LifecycleTransition) -> &'static str {
    match t {
        LifecycleTransition::Running => "running",
        LifecycleTransition::Idle => "idle",
        LifecycleTransition::Interrupted => "interrupted",
        LifecycleTransition::Completed => "done",
        LifecycleTransition::Failed => "failed",
        LifecycleTransition::Killed => "killed",
        LifecycleTransition::AllBackgroundKilled => "all background killed",
    }
}

fn body_text(text: Option<&str>, encrypted: bool) -> String {
    match (text, encrypted) {
        (Some(t), false) => format!("\"{t}\""),
        (_, true) => "[encrypted]".to_string(),
        (None, false) => String::new(),
    }
}

fn joined(parts: &[Option<&str>]) -> String {
    parts
        .iter()
        .flatten()
        .copied()
        .collect::<Vec<_>>()
        .join(", ")
}

/// Timeline row for a selected item kind, or None for items the TUI does not show.
fn timeline_row(e: &TimelineEntry, offset: FixedOffset) -> Option<TimelineRowVm> {
    let row = |icon: &'static str, kind: RowKind, label: &str, text: String| TimelineRowVm {
        time: hhmm(e.ts, offset),
        icon,
        kind,
        label: label.to_string(),
        text,
    };
    Some(match &e.item {
        TimelineItem::User { text } => row("›", RowKind::User, "user", text.clone()),
        TimelineItem::SlashCommand { name, args } => row(
            "›",
            RowKind::Command,
            name,
            args.clone().unwrap_or_default(),
        ),
        TimelineItem::Assistant { text } => {
            if text.is_empty() {
                return None;
            }
            row("•", RowKind::Assistant, "", text.clone())
        }
        TimelineItem::ToolCall { name, summary, .. } => {
            row("▸", RowKind::Tool, name, summary.clone())
        }
        TimelineItem::ToolError { preview } => {
            row("✗", RowKind::ToolError, "error", preview.clone())
        }
        TimelineItem::SubAction {
            summary,
            status,
            exit_code,
        } => {
            let text = match (status, exit_code) {
                (SubActionStatus::Failed, Some(c)) => format!("{summary}  (exit {c})"),
                (SubActionStatus::Failed, None) => format!("{summary}  (failed)"),
                _ => summary.clone(),
            };
            row("↳", RowKind::SubAction, "", text)
        }
        TimelineItem::Message {
            direction,
            peer,
            kind,
            text,
            encrypted,
        } => {
            let body = body_text(text.as_deref(), *encrypted);
            let text = if body.is_empty() {
                message_tag(kind)
            } else {
                format!("{} {body}", message_tag(kind))
            };
            match direction {
                MessageDirection::Incoming => row("←", RowKind::MessageIn, peer, text),
                MessageDirection::Outgoing => row("→", RowKind::MessageOut, peer, text),
            }
        }
        TimelineItem::Spawn {
            child,
            kind,
            agent_type,
            model,
        } => {
            let detail = joined(&[
                Some(kind_name(*kind)),
                agent_type.as_deref(),
                model.as_deref(),
            ]);
            row("⇢", RowKind::Spawn, "spawn", format!("{child} ({detail})"))
        }
        TimelineItem::Lifecycle {
            target,
            transition,
            reason,
        } => {
            let t = transition_name(*transition);
            let text = match reason {
                Some(r) => format!("{t} ({r})"),
                None => t.to_string(),
            };
            row("◇", RowKind::Lifecycle, target, text)
        }
        TimelineItem::Compaction {
            trigger,
            pre_tokens,
            post_tokens,
        } => {
            let tok = |t: &Option<u64>| t.map(tokens).unwrap_or_else(|| "?".to_string());
            let trig = match trigger {
                CompactionTrigger::Auto => "auto",
                CompactionTrigger::Manual => "manual",
                CompactionTrigger::Unknown => "unknown",
            };
            row(
                "⟲",
                RowKind::Compaction,
                "compact",
                format!("{}→{} ({trig})", tok(pre_tokens), tok(post_tokens)),
            )
        }
        TimelineItem::ModelChange { from, to } => {
            let text = match from {
                Some(f) => format!("{f} → {to}"),
                None => to.clone(),
            };
            row("◆", RowKind::ModelChange, "model", text)
        }
        TimelineItem::TurnEnd {
            outcome,
            duration_ms,
        } => {
            let dur = duration_ms.map(duration_label);
            match outcome {
                TurnOutcome::Interrupted => {
                    row("⏹", RowKind::Interrupt, "interrupted", String::new())
                }
                TurnOutcome::Failed { error } => row(
                    "✗",
                    RowKind::TurnEnd,
                    "turn failed",
                    joined(&[error.as_deref(), dur.as_deref()]),
                ),
                TurnOutcome::Completed => {
                    row("─", RowKind::TurnEnd, "turn end", dur.unwrap_or_default())
                }
            }
        }
        TimelineItem::QueueOperation {
            reason, content, ..
        } => {
            // Only prompts absorbed into a running turn are shown (not enqueue/dequeue noise).
            if !reason.as_deref().is_some_and(|r| r.contains("absorbed")) {
                return None;
            }
            row(
                "↲",
                RowKind::Queued,
                "queued",
                content.clone().unwrap_or_default(),
            )
        }
        // Background jobs / hooks / errors are not selected for the TUI.
        TimelineItem::Notification { .. } => return None,
    })
}

// ============================================================================
// Feed
// ============================================================================

fn involves(e: &FeedEntry, id: &AgentId) -> bool {
    e.source == *id || e.from.agent() == Some(id) || e.to.iter().any(|p| p.agent() == Some(id))
}

fn build_feed(
    view: &WorkspaceView,
    ui: &UiState,
    selected: Option<&AgentId>,
    session: Option<&HashSet<&AgentId>>,
) -> Vec<FeedRowVm> {
    // Labels are cached: party_label walks the agent map.
    let mut labels: HashMap<String, String> = HashMap::new();
    let mut label = |p: &FeedParty| -> String {
        let key = format!("{p:?}");
        labels
            .entry(key)
            .or_insert_with(|| view.party_label(p))
            .clone()
    };
    view.feed
        .iter()
        .filter(|e| session.is_none_or(|m| m.contains(&e.source)))
        .filter(|e| match (ui.feed_filter, selected) {
            (FeedFilter::Selected, Some(id)) => involves(e, id),
            _ => true,
        })
        .map(|e| {
            let (kind, tag, text, encrypted) = match &e.kind {
                FeedKind::Message(k) => (
                    FeedRowKind::Message,
                    message_tag(k),
                    if e.encrypted { None } else { e.text.clone() },
                    e.encrypted,
                ),
                FeedKind::Spawn(k) => (
                    FeedRowKind::Spawn,
                    format!("spawn {}", kind_name(*k)),
                    e.text.clone(),
                    false,
                ),
                FeedKind::Lifecycle(t) => (
                    FeedRowKind::Lifecycle,
                    transition_name(*t).to_string(),
                    e.text.clone(),
                    false,
                ),
            };
            FeedRowVm {
                time: hhmm(e.ts, ui.utc_offset),
                kind,
                from: label(&e.from),
                to: e.to.iter().map(&mut label).collect(),
                tag,
                text,
                encrypted,
                involves_selected: selected.is_some_and(|id| involves(e, id)),
            }
        })
        .collect()
}

// ============================================================================
// Status bar
// ============================================================================

fn build_status(
    view: &WorkspaceView,
    ui: &UiState,
    members: &HashSet<&AgentId>,
    visible: &HashSet<AgentId>,
    sessions: &[Session],
    focus: Option<String>,
) -> StatusBarVm {
    let agents: Vec<&AgentView> = members.iter().filter_map(|id| view.agent(id)).collect();
    let count = |s: AgentStatus| agents.iter().filter(|a| a.status == s).count();
    let folded = if ui.filter.is_empty() {
        agents.iter().filter(|a| !visible.contains(a.id())).count()
    } else {
        0
    };
    StatusBarVm {
        scope: match &focus {
            Some(name) => format!("session {name}"),
            None => ui.scope.clone(),
        },
        focus,
        sessions: sessions.len(),
        live: sessions.iter().filter(|s| s.is_live()).count(),
        agents: agents.len(),
        running: count(AgentStatus::Running),
        idle: count(AgentStatus::Idle),
        folded,
        diagnostics: view.total_diagnostic_errors(),
        errors: view.errors.len(),
        last_error: view.errors.last().cloned(),
        feed_filter: ui.feed_filter,
        show_done: ui.show_done,
        auto_select: ui.auto_select,
        collapsed: ui.collapsed.len(),
        filter: ui.filter.clone(),
        filter_editing: ui.filter_editing,
        matches: 0,
    }
}
