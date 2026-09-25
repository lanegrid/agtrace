//! Overview content (top node, session node, folded group): one row per agent
//! with status, context, an activity lane over the chosen window, and what the
//! agent is doing now; session roots get a header line.

use std::collections::HashSet;

use agtrace_sdk::types::AgentId;
use agtrace_sdk::workspace::{AgentStatus, AgentView, Session, WorkspaceView, one_line};
use chrono::{DateTime, Duration, Utc};

use super::sessions::state_vm;
use super::{FoldPlan, TreeRow, badge, ctx, provider_name, status_vm};
use crate::presentation::view_models::watch::{
    FoldedVm, NowVm, OverviewRowVm, OverviewVm, RootHeaderVm, SessionStateVm, UiState,
};

/// Characters kept of the one-line "now" text (the view clips further).
const NOW_TEXT_MAX: usize = 160;

/// Lane geometry: `cells` cells of `cell` each, the last one ending at `end`.
#[derive(Debug, Clone, Copy)]
pub(super) struct Lanes {
    end: DateTime<Utc>,
    cell: Duration,
    cells: usize,
}

impl Lanes {
    /// Cells sized to fit `cols` columns over the UI's window (whole minutes, since
    /// activity is counted per minute); None when there is no room for lanes.
    fn new(
        view: &WorkspaceView,
        ids: &[&AgentId],
        ui: &UiState,
        now: DateTime<Utc>,
    ) -> Option<Self> {
        let cols = ui.viewport.lane_cols as i64;
        if cols == 0 {
            return None;
        }
        let span = match ui.window.secs() {
            Some(s) => s,
            None => ids
                .iter()
                .filter_map(|id| view.agent(id)?.agent.started_at)
                .min()
                .map(|s| (now - s).num_seconds())
                .unwrap_or(0)
                .max(60),
        };
        let per_cell = (span + cols - 1) / cols;
        let cell_secs = ((per_cell + 59) / 60).max(1) * 60;
        let cells = ((span + cell_secs - 1) / cell_secs).clamp(1, cols) as usize;
        let end = DateTime::from_timestamp((now.timestamp() + 59).div_euclid(60) * 60, 0)?;
        Some(Self {
            end,
            cell: Duration::seconds(cell_secs),
            cells,
        })
    }

    fn cell_label(&self) -> String {
        let m = self.cell.num_minutes();
        match (m / 60, m % 60) {
            (0, m) => format!("{m}m"),
            (h, 0) => format!("{h}h"),
            (h, m) => format!("{h}h{m:02}m"),
        }
    }
}

/// Overview of `rows` (agent, label, with a session header); session headers
/// count the agents folded away (`plan`).
pub(super) fn build_overview(
    view: &WorkspaceView,
    ui: &UiState,
    rows: Vec<(TreeRow, String, bool)>,
    sessions: &[Session],
    plan: &FoldPlan,
    now: DateTime<Utc>,
) -> OverviewVm {
    let ids: Vec<&AgentId> = rows.iter().map(|(r, _, _)| &r.id).collect();
    let lanes = Lanes::new(view, &ids, ui, now);
    let rows = rows
        .iter()
        .filter_map(|(r, label, header)| {
            let a = view.agent(&r.id)?;
            let (lane, lane_tones) = lanes.map(|l| lane(a, &l, now)).unwrap_or_default();
            Some(OverviewRowVm {
                id: r.id.as_str().to_string(),
                depth: r.depth,
                guides: r.guides.clone(),
                is_last_sibling: r.is_last,
                label: label.clone(),
                badge: badge(a.agent.kind),
                provider: provider_name(a.agent.provider).to_string(),
                status: status_vm(a.status),
                ctx: ctx(a),
                lane,
                lane_tones,
                now: now_of(a, now),
                root: header.then(|| {
                    let session = sessions.iter().find(|s| s.root == r.id);
                    let folded = if ui.filter.is_empty() {
                        folded(view, &r.id, &plan.visible)
                    } else {
                        FoldedVm::default()
                    };
                    root_header(view, a, session, folded, now)
                }),
            })
        })
        .collect();
    OverviewVm {
        window: ui.window.label().to_string(),
        cell: lanes.map(|l| l.cell_label()).unwrap_or_default(),
        rows,
        older_hidden: 0,
    }
}

/// Finished agents of the session rooted at `root` that are not shown.
fn folded(view: &WorkspaceView, root: &AgentId, visible: &HashSet<AgentId>) -> FoldedVm {
    let mut out = FoldedVm::default();
    for id in view.session_agents(root) {
        if visible.contains(id) {
            continue;
        }
        let Some(a) = view.agent(id) else { continue };
        if a.session_fold.is_some() {
            out.transcripts += 1;
            continue;
        }
        match a.status {
            AgentStatus::Done => out.done += 1,
            AgentStatus::Killed => out.killed += 1,
            _ => {}
        }
    }
    out
}

fn root_header(
    view: &WorkspaceView,
    a: &AgentView,
    session: Option<&Session>,
    folded: FoldedVm,
    now: DateTime<Utc>,
) -> RootHeaderVm {
    let (agents, running) = match session {
        Some(s) => (s.agents, s.running),
        None => {
            let ids = view.session_agents(a.id());
            let running = ids
                .iter()
                .filter(|id| {
                    view.agent(id)
                        .is_some_and(|v| v.status == AgentStatus::Running)
                })
                .count();
            (ids.len(), running)
        }
    };
    RootHeaderVm {
        label: session.map(|s| s.name.clone()).unwrap_or_else(|| a.label()),
        provider: provider_name(a.agent.provider).to_string(),
        bg: session.is_some_and(|s| s.bg),
        state: session
            .map(|s| state_vm(s.state))
            .unwrap_or(SessionStateVm::Older),
        status: status_vm(a.status),
        age_secs: a.agent.started_at.map(|s| (now - s).num_seconds().max(0)),
        model: a.model.clone(),
        effort: a.effort().map(str::to_string),
        ctx: ctx(a),
        compactions: a.detail.compactions,
        agents,
        running,
        folded,
    }
}

/// Density glyph for `events` over `minutes`.
fn density(events: u32, minutes: i64) -> char {
    let rate = events as f64 / minutes.max(1) as f64;
    match rate {
        r if r < 1.0 => '▁',
        r if r < 3.0 => '▂',
        r if r < 6.0 => '▃',
        r if r < 12.0 => '▅',
        _ => '▆',
    }
}

fn tone(s: AgentStatus) -> char {
    match s {
        AgentStatus::Running => 'r',
        AgentStatus::Idle | AgentStatus::Unknown => 'i',
        AgentStatus::Done | AgentStatus::Killed | AgentStatus::Failed => 'd',
    }
}

/// Glyphs and tones of one agent's lane.
fn lane(a: &AgentView, l: &Lanes, now: DateTime<Utc>) -> (String, String) {
    let mut glyphs = String::with_capacity(l.cells * 3);
    let mut tones = String::with_capacity(l.cells);
    let minutes = l.cell.num_minutes();
    // Time of the latest signal: own-log write or reported transition.
    let settled = a
        .last_activity
        .max(a.detail.status_history.last().map(|p| p.at));
    for i in 0..l.cells {
        let to = l.end - l.cell * (l.cells - 1 - i) as i32;
        let from = to - l.cell;
        let existed = a.agent.started_at.is_some_and(|s| s < to);
        if !existed {
            glyphs.push(' ');
            tones.push(' ');
            continue;
        }
        let (mut events, mut compactions) = (0, 0);
        for b in a.detail.activity.between(from, to) {
            events += b.events;
            compactions += b.compactions;
        }
        // Cells after the last signal show the effective status (it may come
        // from the process registry or staleness, which the signal history does
        // not record: a session that went quiet is not "running" until now);
        // earlier cells show the signal history.
        let status = if to >= now || settled.is_none_or(|t| from >= t) {
            Some(a.status)
        } else {
            a.detail.status_history.at(to - Duration::seconds(1))
        };
        let (g, t) = if compactions > 0 {
            ('⟲', 'c')
        } else if events > 0 {
            (
                density(events, minutes),
                tone(status.unwrap_or(AgentStatus::Running)),
            )
        } else if status == Some(AgentStatus::Running) {
            ('·', 'r')
        } else {
            (' ', ' ')
        };
        glyphs.push(g);
        tones.push(t);
    }
    (glyphs, tones)
}

/// Newest of the last assistant text and reasoning (text preferred on a tie).
fn latest_said(a: &AgentView) -> Option<&str> {
    let d = &a.detail;
    match (&d.last_message, &d.last_reasoning) {
        (Some(m), Some(r)) if r.at > m.at => Some(&r.text),
        (Some(m), _) => Some(&m.text),
        (None, Some(r)) => Some(&r.text),
        (None, None) => None,
    }
}

pub(super) fn now_of(a: &AgentView, now: DateTime<Utc>) -> NowVm {
    let ago = |t: Option<DateTime<Utc>>| t.map(|t| (now - t).num_seconds().max(0));
    match a.status {
        AgentStatus::Running => {
            if let Some(t) = &a.current_tool {
                return NowVm::Tool {
                    name: t.name.clone(),
                    summary: t.summary.clone(),
                    elapsed_secs: (now - t.since).num_seconds().max(0),
                };
            }
            if let Some(text) = a.detail.plan.in_progress().and_then(|t| t.doing()) {
                return NowVm::Task {
                    text: one_line(text, NOW_TEXT_MAX),
                };
            }
            match latest_said(a) {
                Some(text) => NowVm::Said {
                    text: one_line(text, NOW_TEXT_MAX),
                },
                None => NowVm::None,
            }
        }
        AgentStatus::Idle => {
            let since = a
                .detail
                .status_history
                .entered(AgentStatus::Idle)
                .or(a.last_active())
                .or(a.last_activity);
            NowVm::Idle {
                secs: ago(since).unwrap_or(0),
            }
        }
        AgentStatus::Done | AgentStatus::Killed | AgentStatus::Failed => {
            let result = a
                .detail
                .result
                .as_ref()
                .filter(|r| !r.encrypted)
                .and_then(|r| r.text.as_deref());
            match result {
                Some(text) if a.status == AgentStatus::Done => NowVm::Result {
                    text: one_line(text, NOW_TEXT_MAX),
                },
                _ => {
                    let ended = a
                        .detail
                        .status_history
                        .last()
                        .filter(|p| p.status.is_terminal())
                        .map(|p| p.at)
                        .or(a.last_activity);
                    NowVm::Ended {
                        status: status_vm(a.status),
                        secs: ago(ended),
                        reason: a.detail.end_reason.clone(),
                    }
                }
            }
        }
        AgentStatus::Unknown => NowVm::None,
    }
}
