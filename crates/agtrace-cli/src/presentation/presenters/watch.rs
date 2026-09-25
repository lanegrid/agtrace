//! Presenter of the multi-agent watch TUI (design §6.3).
//!
//! [`build_screen`] is pure: `(&WorkspaceView, &UiState, now) -> WatchScreenVm`.
//! It decides what is shown (requirements "selected" items only):
//! - navigator: the scope, its sessions (live first, older ones in one group) and
//!   each session's agent tree, a parent's finished children folded into a group;
//! - content of the selected node: the multi-session overview (top), a session's
//!   overview, an agent's detail (instructions, now, result, timeline), the items
//!   of a folded group, or the older sessions;
//! - feed: inter-agent messages, spawns and lifecycle changes scoped to the
//!   selection (encrypted bodies are never shown, only type and route).

mod detail;
mod navigator;
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
    ActivityVm, AgentRowVm, AgentTimelineVm, ConsoleVm, ContentVm, CtxVm, FeedRowKind, FeedRowVm,
    FoldedVm, KeyedRow, NAV_FOLD_MIN, NavKind, RowKind, StatusBarVm, StatusVm, TimelineRowVm,
    UiState, WatchScreenVm,
};

/// Build the whole screen from the current workspace snapshot.
///
/// Toast expiry is judged against the monotonic clock (the workspace clock `now`
/// may be frozen for fixtures).
pub fn build_screen(view: &WorkspaceView, ui: &UiState, now: DateTime<Utc>) -> WatchScreenVm {
    let filter = ui.filter.to_lowercase();
    let all_sessions = view.sessions(now);
    let roots: Vec<AgentId> = all_sessions
        .iter()
        .filter(|s| s.has_transcript)
        .map(|s| s.root.clone())
        .collect();
    let plan = fold_plan(view, &roots, ui.show_done, &filter);
    let nav = navigator::build(
        view,
        ui,
        &all_sessions,
        &plan,
        &filter,
        top_label(&ui.scope),
    );
    let top = nav.rows.first().expect("the top node is always there");
    let sel = nav.rows.iter().find(|r| r.selected).unwrap_or(top).clone();

    let session_of = |id: &AgentId| {
        all_sessions
            .iter()
            .find(|s| s.root == *view.session_root(id))
    };
    let (content, overview, detail, feed_scope) = match sel.kind {
        NavKind::Top => {
            // The compact overview leaves the older sessions to the navigator's
            // group (only when there is something newer).
            let newer = all_sessions.iter().any(|s| s.state != SessionState::Older);
            let shown: Vec<&Session> = all_sessions
                .iter()
                .filter(|s| s.has_transcript && plan.visible.contains(&s.root))
                .filter(|s| !newer || s.state != SessionState::Older)
                .collect();
            let rows = session_rows(view, &shown, &plan);
            let mut ov = overview::build_overview(view, ui, rows, &all_sessions, &plan, now);
            if newer {
                ov.older_hidden = all_sessions
                    .iter()
                    .filter(|s| s.state == SessionState::Older)
                    .count();
            }
            (ContentVm::Overview, Some(ov), None, FeedScope::All)
        }
        NavKind::Session => {
            let s = all_sessions.iter().find(|s| s.root.as_str() == sel.key);
            let has_transcript = s.is_some_and(|s| s.has_transcript);
            let root = AgentId::parse(&sel.key);
            match root.as_ref().and_then(|id| view.agent(id)) {
                Some(a) if ui.root_detail => (
                    ContentVm::Agent {
                        id: sel.key.clone(),
                    },
                    None,
                    Some(detail::build_detail(view, ui, a, now)),
                    FeedScope::Sources(members(view, a.id())),
                ),
                _ => {
                    let shown: Vec<&Session> = s.filter(|s| s.has_transcript).into_iter().collect();
                    let rows = session_rows(view, &shown, &plan);
                    let ov = overview::build_overview(view, ui, rows, &all_sessions, &plan, now);
                    let scope = match &root {
                        Some(r) if has_transcript => FeedScope::Sources(members(view, r)),
                        _ => FeedScope::Sources(HashSet::new()),
                    };
                    (
                        ContentVm::Session {
                            id: sel.key.clone(),
                            has_transcript,
                        },
                        Some(ov),
                        None,
                        scope,
                    )
                }
            }
        }
        NavKind::Agent => {
            let a = AgentId::parse(&sel.key).and_then(|id| view.agent(&id));
            match a {
                Some(a) => (
                    ContentVm::Agent {
                        id: sel.key.clone(),
                    },
                    None,
                    Some(detail::build_detail(view, ui, a, now)),
                    FeedScope::Agent(a.id().clone()),
                ),
                None => (ContentVm::Overview, None, None, FeedScope::All),
            }
        }
        NavKind::Fold => {
            let parent = sel.parent.clone().unwrap_or_default();
            let items = AgentId::parse(&parent)
                .and_then(|p| plan.groups.get(&p))
                .cloned()
                .unwrap_or_default();
            let mut rows = Vec::new();
            let mut seen = HashSet::new();
            for (i, id) in items.iter().enumerate() {
                walk(
                    view,
                    id,
                    0,
                    &mut Vec::new(),
                    i + 1 == items.len(),
                    &|_| true,
                    &mut rows,
                    &mut seen,
                );
            }
            let sources: HashSet<AgentId> = rows.iter().map(|r| r.id.clone()).collect();
            let rows = rows
                .into_iter()
                .filter_map(|r| {
                    let label = tree_label(view, view.agent(&r.id)?);
                    Some((r, label, false))
                })
                .collect();
            let ov = overview::build_overview(view, ui, rows, &all_sessions, &plan, now);
            (
                ContentVm::Fold {
                    parent,
                    folded: sel.folded.unwrap_or_default(),
                },
                Some(ov),
                None,
                FeedScope::Sources(sources),
            )
        }
        NavKind::Older => {
            let older: Vec<&Session> = all_sessions
                .iter()
                .filter(|s| s.state == SessionState::Older)
                .collect();
            let sources = older
                .iter()
                .filter(|s| s.has_transcript)
                .flat_map(|s| members(view, &s.root))
                .collect();
            (
                ContentVm::Older { count: older.len() },
                None,
                None,
                FeedScope::Sources(sources),
            )
        }
    };

    let feed = build_feed(view, ui, &feed_scope);
    let feed_scope = match &feed_scope {
        FeedScope::All => "all".to_string(),
        FeedScope::Agent(id) => view.agent(id).map(|a| a.label()).unwrap_or_default(),
        FeedScope::Sources(_) => match sel.kind {
            NavKind::Session => "this session".to_string(),
            NavKind::Fold => "folded agents".to_string(),
            NavKind::Older => "older sessions".to_string(),
            _ => "this session".to_string(),
        },
    };

    // Breadcrumb: scope › session › agent / group.
    let mut crumbs = vec![top.label.clone()];
    match sel.kind {
        NavKind::Top => {}
        NavKind::Session => crumbs.push(sel.label.clone()),
        NavKind::Agent | NavKind::Fold => {
            let anchor = match sel.kind {
                NavKind::Fold => sel.parent.as_deref().and_then(AgentId::parse),
                _ => AgentId::parse(&sel.key),
            };
            if let Some(s) = anchor.as_ref().and_then(session_of) {
                crumbs.push(s.name.clone());
            }
            match sel.kind {
                NavKind::Agent => crumbs.push(sel.label.clone()),
                _ => {
                    if let Some(p) = anchor
                        .as_ref()
                        .filter(|p| view.session_root(p) != *p)
                        .and_then(|p| view.agent(p))
                    {
                        crumbs.push(p.label());
                    }
                    crumbs.push(sel.label.clone());
                }
            }
        }
        NavKind::Older => crumbs.push(sel.label.clone()),
    }

    let sessions = sessions::build_sessions(view, ui, &all_sessions, now);
    let mut status = build_status(view, ui, &roots, &plan, &all_sessions, crumbs);
    if !filter.is_empty() {
        status.matches = nav.rows.iter().filter(|r| r.matched).count();
    }

    WatchScreenVm {
        nav,
        content,
        sessions,
        overview,
        detail,
        feed,
        feed_scope,
        status,
        focus_pane: ui.focus,
        content_scroll: ui.content_scroll,
        feed_scroll: ui.feed_scroll,
        nav_hidden: ui.nav_hidden,
        show_help: ui.show_help,
        toast: ui
            .toast
            .as_ref()
            .filter(|t| t.is_live(Instant::now()))
            .map(|t| t.text.clone()),
    }
}

/// Name of the top node: the scope without `project` and the `since` window
/// (`yohaku-studio`, `all projects`, `session 01a07053`).
fn top_label(scope: &str) -> String {
    let base = scope.split(" · since ").next().unwrap_or(scope);
    let base = base.strip_prefix("project ").unwrap_or(base);
    if base.is_empty() {
        "workspace".to_string()
    } else {
        base.to_string()
    }
}

/// Agents of the session rooted at `root`.
fn members(view: &WorkspaceView, root: &AgentId) -> HashSet<AgentId> {
    view.session_agents(root).into_iter().cloned().collect()
}

/// Overview rows of `sessions`: each root (named like its session, with a header)
/// and its agents shown in place, in tree order.
fn session_rows(
    view: &WorkspaceView,
    sessions: &[&Session],
    plan: &FoldPlan,
) -> Vec<(TreeRow, String, bool)> {
    let mut rows = Vec::new();
    let mut seen = HashSet::new();
    for s in sessions {
        let mut tree = Vec::new();
        let include = |id: &AgentId| plan.visible.contains(id);
        walk(
            view,
            &s.root,
            0,
            &mut Vec::new(),
            true,
            &include,
            &mut tree,
            &mut seen,
        );
        for r in tree {
            let Some(a) = view.agent(&r.id) else { continue };
            let root = r.depth == 0;
            let label = if root {
                s.name.clone()
            } else {
                tree_label(view, a)
            };
            rows.push((r, label, root));
        }
    }
    rows
}

/// Build the console (line printer) snapshot: every session's agent tree (nothing
/// folded), every agent's timeline and the workspace feed, keyed by event id so
/// the printer emits each row once (design §6.3, `watch --mode console`).
pub fn build_console(view: &WorkspaceView, ui: &UiState, now: DateTime<Utc>) -> ConsoleVm {
    let mut rows = Vec::new();
    let mut seen = HashSet::new();
    for s in view.sessions(now).iter().filter(|s| s.has_transcript) {
        walk(
            view,
            &s.root,
            0,
            &mut Vec::new(),
            true,
            &|_| true,
            &mut rows,
            &mut seen,
        );
    }
    let tree: Vec<AgentRowVm> = rows
        .iter()
        .filter_map(|r| {
            let a = view.agent(&r.id)?;
            Some(AgentRowVm {
                id: r.id.as_str().to_string(),
                depth: r.depth,
                label: tree_label(view, a),
                provider: provider_name(a.agent.provider).to_string(),
                badge: badge(a.agent.kind),
                status: status_vm(a.status),
                ctx_pct: ctx(a).map(|c| c.pct),
            })
        })
        .collect();
    let timelines = tree
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
    // The unscoped feed stays aligned with `view.feed`.
    let feed = build_feed(view, ui, &FeedScope::All);
    let feed_keys = view.feed.iter().map(|e| e.event_id.to_string()).collect();
    ConsoleVm {
        tree,
        feed,
        timelines,
        feed_keys,
    }
}

// ============================================================================
// Trees and folds
// ============================================================================

/// Agents that the done fold hides: finished ones (failed ones stay visible),
/// and other transcripts of the session (earlier, `/clear` stubs) unless running.
fn is_foldable(a: &AgentView) -> bool {
    matches!(a.status, AgentStatus::Done | AgentStatus::Killed)
        || (a.session_fold.is_some() && a.status != AgentStatus::Running)
}

/// Which agents are shown in place, and which are folded into groups.
pub(super) struct FoldPlan {
    /// Agents shown in place (roots always, unless a `/` filter leaves them out).
    pub visible: HashSet<AgentId>,
    /// Parent → its finished children folded into one navigator group (tree order).
    pub groups: HashMap<AgentId, Vec<AgentId>>,
}

/// Fold finished subtrees below the session roots: a parent's finished children
/// (whose subtrees are finished too) form one group when there are at least
/// [`NAV_FOLD_MIN`] of them; a single one stays in place. `d` (`show_done`) shows
/// everything; a `/` filter (lowercase) shows its matches and their ancestors.
fn fold_plan(view: &WorkspaceView, roots: &[AgentId], show_done: bool, filter: &str) -> FoldPlan {
    if !filter.is_empty() || show_done {
        return FoldPlan {
            visible: visibility(view, roots, false, filter),
            groups: HashMap::new(),
        };
    }
    let mut visible = visibility(view, roots, true, "");
    let mut groups = HashMap::new();
    let parents: Vec<AgentId> = visible.iter().cloned().collect();
    for p in parents {
        let Some(a) = view.agent(&p) else { continue };
        let folded: Vec<AgentId> = a
            .children
            .iter()
            .filter(|c| !visible.contains(*c) && view.agent(c).is_some())
            .cloned()
            .collect();
        if folded.is_empty() {
            continue;
        }
        if folded.len() >= NAV_FOLD_MIN {
            groups.insert(p, folded);
        } else {
            for c in &folded {
                restore(view, c, &mut visible);
            }
        }
    }
    FoldPlan { visible, groups }
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

/// Counts of the finished agents (and earlier transcripts) in the subtrees of `ids`.
fn folded_counts(view: &WorkspaceView, ids: &[AgentId]) -> FoldedVm {
    let mut out = FoldedVm::default();
    let mut stack: Vec<&AgentId> = ids.iter().collect();
    let mut seen = HashSet::new();
    while let Some(id) = stack.pop() {
        if !seen.insert(id) {
            continue;
        }
        let Some(a) = view.agent(id) else { continue };
        if a.session_fold.is_some() {
            out.transcripts += 1;
        } else {
            match a.status {
                AgentStatus::Done => out.done += 1,
                AgentStatus::Killed => out.killed += 1,
                _ => {}
            }
        }
        stack.extend(a.children.iter());
    }
    out
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

/// Agents shown below `roots`: all but the finished subtrees (`fold`; kept when a
/// descendant is shown); with a `/` filter (lowercase, non-empty), only the
/// matching agents and their ancestors (folds ignored: finished agents are found).
fn visibility(
    view: &WorkspaceView,
    roots: &[AgentId],
    fold: bool,
    filter: &str,
) -> HashSet<AgentId> {
    /// Returns whether `id` is shown.
    fn visit(
        view: &WorkspaceView,
        id: &AgentId,
        depth: usize,
        fold: bool,
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
        let mut any_child = false;
        for c in &a.children {
            any_child |= visit(view, c, depth + 1, fold, filter, out, seen);
        }
        let shown = if filter.is_empty() {
            depth == 0 || any_child || !fold || !is_foldable(a)
        } else {
            any_child || matches_filter(view, a, filter)
        };
        if shown {
            out.insert(id.clone());
        }
        shown
    }
    let mut out = HashSet::new();
    let mut seen = HashSet::new();
    for r in roots {
        visit(view, r, 0, fold, filter, &mut out, &mut seen);
    }
    out
}

/// One agent in a tree walk.
#[derive(Debug, Clone)]
pub(super) struct TreeRow {
    pub id: AgentId,
    pub depth: u16,
    /// Per ancestor level below the walk's top: a vertical guide continues.
    pub guides: Vec<bool>,
    pub is_last: bool,
}

/// Depth-first walk from `id` over the children `include` accepts.
#[allow(clippy::too_many_arguments)]
fn walk(
    view: &WorkspaceView,
    id: &AgentId,
    depth: u16,
    guides: &mut Vec<bool>,
    is_last: bool,
    include: &dyn Fn(&AgentId) -> bool,
    out: &mut Vec<TreeRow>,
    seen: &mut HashSet<AgentId>,
) {
    if !seen.insert(id.clone()) {
        return;
    }
    let Some(a) = view.agent(id) else { return };
    out.push(TreeRow {
        id: id.clone(),
        depth,
        guides: guides.clone(),
        is_last,
    });
    let children: Vec<&AgentId> = a.children.iter().filter(|c| include(c)).collect();
    if depth > 0 {
        guides.push(!is_last);
    }
    let n = children.len();
    for (i, c) in children.into_iter().enumerate() {
        walk(view, c, depth + 1, guides, i + 1 == n, include, out, seen);
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

/// Which feed entries the selection shows.
enum FeedScope {
    /// Top node: everything in scope.
    All,
    /// Entries emitted by, sent by or addressed to one of these agents (a
    /// session, a folded group, the older sessions).
    Sources(HashSet<AgentId>),
    /// Entries sent / received / emitted by one agent.
    Agent(AgentId),
}

fn involves(e: &FeedEntry, id: &AgentId) -> bool {
    e.source == *id || e.from.agent() == Some(id) || e.to.iter().any(|p| p.agent() == Some(id))
}

fn build_feed(view: &WorkspaceView, ui: &UiState, scope: &FeedScope) -> Vec<FeedRowVm> {
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
        .filter(|e| match scope {
            FeedScope::All => true,
            FeedScope::Sources(ids) => {
                ids.contains(&e.source)
                    || e.from.agent().is_some_and(|a| ids.contains(a))
                    || e.to
                        .iter()
                        .any(|p| p.agent().is_some_and(|a| ids.contains(a)))
            }
            FeedScope::Agent(id) => involves(e, id),
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
    roots: &[AgentId],
    plan: &FoldPlan,
    sessions: &[Session],
    crumbs: Vec<String>,
) -> StatusBarVm {
    let members: HashSet<&AgentId> = roots.iter().flat_map(|r| view.session_agents(r)).collect();
    let agents: Vec<&AgentView> = members.iter().filter_map(|id| view.agent(id)).collect();
    let count = |s: AgentStatus| agents.iter().filter(|a| a.status == s).count();
    let folded = if ui.filter.is_empty() {
        agents
            .iter()
            .filter(|a| !plan.visible.contains(a.id()))
            .count()
    } else {
        0
    };
    StatusBarVm {
        crumbs,
        sessions: sessions.len(),
        live: sessions.iter().filter(|s| s.is_live()).count(),
        agents: agents.len(),
        running: count(AgentStatus::Running),
        idle: count(AgentStatus::Idle),
        folded,
        diagnostics: view.total_diagnostic_errors(),
        errors: view.errors.len(),
        last_error: view.errors.last().cloned(),
        show_done: ui.show_done,
        auto_select: ui.auto_select,
        filter: ui.filter.clone(),
        filter_editing: ui.filter_editing,
        matches: 0,
    }
}
