//! Navigator: the always-visible tree of the scope, its sessions and their agents.
//!
//! ```text
//! ◆ yohaku-studio  3 live            top node (scope)
//! ▸ ● PR 1693 の継続                  session (collapsed)
//! ▾ ○ Projects制作のボトルネック特定     session (expanded)
//!   ├ ✓ S explore call sites          agent
//!   └ ▸ ⊘ 20 killed (d)               folded group
//! ▸ ✓ 1 older session                 older sessions
//! ```
//!
//! Sessions are ordered by liveness (busy, idle, recent); older ones collapse into
//! one group while something newer exists. Below a session, its root agent's
//! children in tree order; a parent's finished children fold into one group node
//! at the end (see [`super::FoldPlan`]). Selection is resolved here: the selected
//! key, else (hidden by a collapse or fold) its nearest shown ancestor or group,
//! else the top node.

use std::collections::HashSet;

use agtrace_sdk::types::AgentId;
use agtrace_sdk::workspace::{Session, SessionState, WorkspaceView};

use super::sessions::state_vm;
use super::{
    FoldPlan, badge, ctx, folded_counts, matches_filter, provider_name, status_vm, tree_label,
};
use crate::presentation::view_models::watch::{
    FoldedVm, NAV_OLDER, NAV_TOP, NavKind, NavRowVm, NavVm, SessionStateVm, StatusVm, UiState,
    fold_key,
};

struct Builder<'a> {
    view: &'a WorkspaceView,
    ui: &'a UiState,
    plan: &'a FoldPlan,
    filter: &'a str,
    /// Sessions open by default (the only one in view).
    sessions_open: bool,
    rows: Vec<NavRowVm>,
    seen: HashSet<AgentId>,
}

/// Session liveness as an agent-like status glyph.
fn session_status(s: SessionStateVm) -> StatusVm {
    match s {
        SessionStateVm::Busy => StatusVm::Running,
        SessionStateVm::Idle => StatusVm::Idle,
        SessionStateVm::Recent | SessionStateVm::Older => StatusVm::Done,
    }
}

/// `⊘ 20 killed · ✓ 3 done · 1 earlier (d)`.
pub(super) fn fold_label(f: FoldedVm) -> String {
    let mut parts = Vec::new();
    if f.killed > 0 {
        parts.push(format!("⊘ {} killed", f.killed));
    }
    if f.done > 0 {
        parts.push(format!("✓ {} done", f.done));
    }
    if f.transcripts > 0 {
        parts.push(format!("{} earlier", f.transcripts));
    }
    format!("{} (d)", parts.join(" · "))
}

impl Builder<'_> {
    fn open(&self, key: &str, default: bool) -> bool {
        if !self.filter.is_empty() {
            return true;
        }
        self.ui.open.get(key).copied().unwrap_or(default)
    }

    fn row(&self, key: String, kind: NavKind, depth: u16, parent: &str) -> NavRowVm {
        NavRowVm {
            key,
            kind,
            depth,
            guides: Vec::new(),
            is_last_sibling: true,
            parent: Some(parent.to_string()),
            label: String::new(),
            provider: String::new(),
            badge: None,
            status: StatusVm::Unknown,
            state: None,
            bg: false,
            live: 0,
            folded: None,
            ctx_pct: None,
            expandable: false,
            expanded: false,
            first_item: None,
            matched: false,
            selected: false,
        }
    }

    /// Children of `parent` in place (plus its folded group, last), or every child
    /// inside an expanded group (`all`).
    fn children(&self, parent: &AgentId, all: bool) -> (Vec<AgentId>, bool) {
        let Some(a) = self.view.agent(parent) else {
            return (Vec::new(), false);
        };
        let kids = a
            .children
            .iter()
            .filter(|c| all || self.plan.visible.contains(*c))
            .cloned()
            .collect();
        (kids, !all && self.plan.groups.contains_key(parent))
    }

    fn session(&mut self, s: &Session, depth: u16, parent: &str, is_last: bool, guides: &[bool]) {
        let key = s.root.as_str().to_string();
        let root = self.view.agent(&s.root);
        let (kids, group) = self.children(&s.root, false);
        let expandable = !kids.is_empty() || group;
        let expanded = expandable && self.open(&key, self.sessions_open);
        let state = state_vm(s.state);
        let matched = !self.filter.is_empty()
            && (s.name.to_lowercase().contains(self.filter)
                || root.is_some_and(|a| matches_filter(self.view, a, self.filter)));
        let mut row = self.row(key, NavKind::Session, depth, parent);
        row.guides = guides.to_vec();
        row.is_last_sibling = is_last;
        row.label = s.name.clone();
        row.provider = provider_name(s.provider).to_string();
        row.status = session_status(state);
        row.state = Some(state);
        row.bg = s.bg;
        row.ctx_pct = root.and_then(ctx).map(|c| c.pct);
        row.expandable = expandable;
        row.expanded = expanded;
        row.matched = matched;
        self.rows.push(row);
        self.seen.insert(s.root.clone());
        if expanded {
            let mut guides = guides.to_vec();
            if depth >= 2 {
                guides.push(!is_last);
            }
            self.below(&s.root, kids, group, depth + 1, &mut guides, false);
        }
    }

    /// Rows of `kids` (and the folded group of `parent`) at `depth`.
    fn below(
        &mut self,
        parent: &AgentId,
        kids: Vec<AgentId>,
        group: bool,
        depth: u16,
        guides: &mut Vec<bool>,
        all: bool,
    ) {
        let n = kids.len() + usize::from(group);
        for (i, c) in kids.iter().enumerate() {
            self.agent(c, parent.as_str(), depth, guides, i + 1 == n, all);
        }
        if group {
            self.group(parent, depth, guides);
        }
    }

    fn agent(
        &mut self,
        id: &AgentId,
        parent: &str,
        depth: u16,
        guides: &mut Vec<bool>,
        is_last: bool,
        all: bool,
    ) {
        if !self.seen.insert(id.clone()) {
            return;
        }
        let Some(a) = self.view.agent(id) else { return };
        let key = id.as_str().to_string();
        let (kids, group) = self.children(id, all);
        let expandable = !kids.is_empty() || group;
        let expanded = expandable && self.open(&key, true);
        let mut row = self.row(key, NavKind::Agent, depth, parent);
        row.guides = guides.clone();
        row.is_last_sibling = is_last;
        row.label = tree_label(self.view, a);
        row.provider = provider_name(a.agent.provider).to_string();
        row.badge = badge(a.agent.kind);
        row.status = status_vm(a.status);
        row.ctx_pct = ctx(a).map(|c| c.pct);
        row.expandable = expandable;
        row.expanded = expanded;
        row.matched = !self.filter.is_empty() && matches_filter(self.view, a, self.filter);
        self.rows.push(row);
        if expanded {
            guides.push(!is_last);
            self.below(id, kids, group, depth + 1, guides, all);
            guides.pop();
        }
    }

    /// The folded group of `parent` (always its last child).
    fn group(&mut self, parent: &AgentId, depth: u16, guides: &mut Vec<bool>) {
        let items = self.plan.groups.get(parent).cloned().unwrap_or_default();
        let key = fold_key(parent.as_str());
        let expanded = self.open(&key, false);
        let folded = folded_counts(self.view, &items);
        let mut row = self.row(key.clone(), NavKind::Fold, depth, parent.as_str());
        row.guides = guides.clone();
        row.label = fold_label(folded);
        row.status = if folded.killed > 0 {
            StatusVm::Killed
        } else {
            StatusVm::Done
        };
        row.folded = Some(folded);
        row.expandable = true;
        row.expanded = expanded;
        row.first_item = items.first().map(|i| i.as_str().to_string());
        self.rows.push(row);
        if expanded {
            // Items hang below the group node; their keys stay agent ids.
            guides.push(false);
            let n = items.len();
            for (i, c) in items.iter().enumerate() {
                self.agent(c, &key, depth + 1, guides, i + 1 == n, true);
            }
            guides.pop();
        }
    }
}

/// Navigator rows with the selection resolved (exactly one row is selected).
pub(super) fn build(
    view: &WorkspaceView,
    ui: &UiState,
    sessions: &[Session],
    plan: &FoldPlan,
    filter: &str,
    top_label: String,
) -> NavVm {
    // A session is listed while the filter keeps it: its root (or a descendant)
    // matches, or (no transcript) its name does.
    let kept = |s: &Session| {
        if filter.is_empty() {
            return true;
        }
        if s.has_transcript {
            plan.visible.contains(&s.root)
        } else {
            s.name.to_lowercase().contains(filter)
        }
    };
    let newer = sessions.iter().any(|s| s.state != SessionState::Older);
    let (main, older): (Vec<&Session>, Vec<&Session>) = sessions
        .iter()
        .filter(|s| kept(s))
        .partition(|s| !newer || s.state != SessionState::Older);
    let mut b = Builder {
        view,
        ui,
        plan,
        filter,
        sessions_open: main.len() == 1,
        rows: Vec::new(),
        seen: HashSet::new(),
    };
    let mut top = b.row(NAV_TOP.to_string(), NavKind::Top, 0, "");
    top.parent = None;
    top.label = top_label;
    top.live = sessions.iter().filter(|s| s.is_live()).count();
    top.expandable = true;
    top.expanded = true;
    b.rows.push(top);

    let n = main.len() + usize::from(!older.is_empty());
    for (i, s) in main.iter().enumerate() {
        b.session(s, 1, NAV_TOP, i + 1 == n, &[]);
    }
    if !older.is_empty() {
        let expanded = b.open(NAV_OLDER, false);
        let mut row = b.row(NAV_OLDER.to_string(), NavKind::Older, 1, NAV_TOP);
        let noun = if older.len() == 1 {
            "session"
        } else {
            "sessions"
        };
        row.label = format!("{} older {noun}", older.len());
        row.status = StatusVm::Done;
        row.expandable = true;
        row.expanded = expanded;
        row.first_item = older.first().map(|s| s.root.as_str().to_string());
        b.rows.push(row);
        if expanded {
            for (i, s) in older.iter().enumerate() {
                b.session(s, 2, NAV_OLDER, i + 1 == older.len(), &[]);
            }
        }
    }

    let mut rows = b.rows;
    let older_roots: HashSet<&AgentId> = older.iter().map(|s| &s.root).collect();
    let key = selection(view, ui, plan, filter, &rows, &older_roots);
    for r in &mut rows {
        r.selected = r.key == key;
    }
    NavVm { rows }
}

/// Selected key: the most recently active agent shown (auto-select), else the
/// remembered key when shown, else (filtering) the first match, else the nearest
/// shown ancestor or folded group, else the top node.
fn selection(
    view: &WorkspaceView,
    ui: &UiState,
    plan: &FoldPlan,
    filter: &str,
    rows: &[NavRowVm],
    older_roots: &HashSet<&AgentId>,
) -> String {
    let shown: HashSet<&str> = rows.iter().map(|r| r.key.as_str()).collect();
    if ui.auto_select {
        let best = rows
            .iter()
            .filter(|r| matches!(r.kind, NavKind::Session | NavKind::Agent))
            .filter_map(|r| {
                let id = AgentId::parse(&r.key)?;
                let t = view.agent(&id)?.last_activity?;
                Some((t, r.key.as_str()))
            })
            .max_by(|x, y| x.0.cmp(&y.0).then_with(|| y.1.cmp(x.1)));
        if let Some((_, key)) = best {
            return key.to_string();
        }
    }
    let first_match = || {
        rows.iter()
            .find(|r| r.matched)
            .map(|r| r.key.clone())
            .unwrap_or_else(|| NAV_TOP.to_string())
    };
    let Some(want) = ui.selected.as_deref() else {
        return if filter.is_empty() {
            NAV_TOP.to_string()
        } else {
            first_match()
        };
    };
    if shown.contains(want) {
        return want.to_string();
    }
    if !filter.is_empty() {
        return first_match();
    }
    // Climb from the agent (or a vanished group's parent) to what is shown.
    let start = want.strip_prefix("@fold:").unwrap_or(want);
    let Some(mut cur) = AgentId::parse(start) else {
        return NAV_TOP.to_string();
    };
    for _ in 0..64 {
        let parent = view.agent(&cur).and_then(|a| a.tree_parent.clone());
        if let Some(p) = &parent {
            let key = fold_key(p.as_str());
            if plan.groups.get(p).is_some_and(|g| g.contains(&cur)) && shown.contains(key.as_str())
            {
                return key;
            }
            if shown.contains(p.as_str()) {
                return p.as_str().to_string();
            }
        }
        match parent {
            Some(p) => cur = p,
            None => break,
        }
    }
    if older_roots.contains(&cur) && shown.contains(NAV_OLDER) {
        return NAV_OLDER.to_string();
    }
    NAV_TOP.to_string()
}
