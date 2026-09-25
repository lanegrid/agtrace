//! Sessions list (`0` screen and the overview's summary block).

use agtrace_sdk::types::AgentId;
use agtrace_sdk::workspace::{
    AgentStatus, Session, SessionNameSource, SessionState, WorkspaceView,
};
use chrono::{DateTime, Utc};

use super::overview::now_of;
use super::{ctx, provider_name};
use crate::presentation::view_models::watch::{
    NowVm, SessionRowVm, SessionStateVm, SessionsVm, UiState,
};

/// What the session does now: the root's work while it runs, else that of its
/// most recently active running agent, else the root's state.
fn session_now(view: &WorkspaceView, root: &AgentId, now: DateTime<Utc>) -> NowVm {
    let Some(r) = view.agent(root) else {
        return NowVm::None;
    };
    if r.status == AgentStatus::Running {
        return now_of(r, now);
    }
    view.session_agents(root)
        .into_iter()
        .filter_map(|id| view.agent(id))
        .filter(|a| a.status == AgentStatus::Running)
        .max_by_key(|a| (a.last_activity, a.id().clone()))
        .map(|a| now_of(a, now))
        .unwrap_or_else(|| now_of(r, now))
}

pub(super) fn state_vm(s: SessionState) -> SessionStateVm {
    match s {
        SessionState::Busy => SessionStateVm::Busy,
        SessionState::Idle => SessionStateVm::Idle,
        SessionState::Recent => SessionStateVm::Recent,
        SessionState::Older => SessionStateVm::Older,
    }
}

/// Rows of every session (older ones folded unless `show_older`); the cursor is
/// the remembered one when listed, else the focused session, else the first row.
pub(super) fn build_sessions(
    view: &WorkspaceView,
    ui: &UiState,
    sessions: &[Session],
    focus: Option<&AgentId>,
    now: DateTime<Utc>,
) -> SessionsVm {
    let count = |f: fn(SessionState) -> bool| sessions.iter().filter(|s| f(s.state)).count();
    let live = count(SessionState::is_live);
    let recent = count(|s| s == SessionState::Recent);
    let older = count(|s| s == SessionState::Older);
    let listed: Vec<&Session> = sessions
        .iter()
        .filter(|s| ui.show_older || s.state != SessionState::Older)
        .collect();
    let listed_id = |id: &str| listed.iter().any(|s| s.root.as_str() == id);
    let cursor = ui
        .session_cursor
        .as_deref()
        .filter(|id| listed_id(id))
        .map(str::to_string)
        .or_else(|| {
            focus
                .map(|f| f.as_str().to_string())
                .filter(|id| listed_id(id))
        })
        .or_else(|| listed.first().map(|s| s.root.as_str().to_string()));
    let rows = listed
        .into_iter()
        .map(|s| {
            let root = view.agent(&s.root);
            SessionRowVm {
                id: s.root.as_str().to_string(),
                provider: provider_name(s.provider).to_string(),
                name: s.name.clone(),
                name_is_id: s.name_source == SessionNameSource::Id,
                short_id: s.short_id.clone(),
                bg: s.bg,
                state: state_vm(s.state),
                has_transcript: s.has_transcript,
                agents: s.agents,
                running: s.running,
                ctx: root.and_then(ctx),
                last_secs: s.last_activity.map(|t| (now - t).num_seconds().max(0)),
                now: session_now(view, &s.root, now),
                selected: cursor.as_deref() == Some(s.root.as_str()),
                focused: focus == Some(&s.root),
            }
        })
        .collect();
    SessionsVm {
        scope: ui.scope.clone(),
        rows,
        live,
        recent,
        older,
        older_folded: if ui.show_older { 0 } else { older },
        focus: focus.map(|f| f.as_str().to_string()),
    }
}
