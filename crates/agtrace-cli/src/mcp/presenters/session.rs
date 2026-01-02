use agtrace_sdk::types::AgentSession;

use crate::mcp::models::common::{ContentLevel, ResponseMeta};
use crate::mcp::models::response::{
    ListSessionsViewModel, SessionFullViewModel, SessionSummaryDto, SessionSummaryViewModel,
    SessionTurnsViewModel, TurnWithIndex,
};

pub fn present_list_sessions(
    sessions: Vec<agtrace_sdk::SessionSummary>,
    next_cursor: Option<String>,
) -> ListSessionsViewModel {
    let total_in_page = sessions.len();
    ListSessionsViewModel {
        sessions: sessions.into_iter().map(SessionSummaryDto).collect(),
        total_in_page,
        next_cursor,
    }
}

pub fn present_session_summary(
    session: AgentSession,
    metadata: Option<agtrace_sdk::types::SessionMetadata>,
) -> SessionSummaryViewModel {
    let (project_hash, provider) = metadata
        .map(|m| (Some(m.project_hash.to_string()), Some(m.provider)))
        .unwrap_or((None, None));

    let mut vm = SessionSummaryViewModel {
        session,
        project_hash,
        provider,
        _meta: ResponseMeta::from_bytes(0),
    };

    if let Ok(json) = serde_json::to_string(&vm) {
        let bytes = json.len();
        vm._meta = ResponseMeta::from_bytes(bytes).with_content_level(ContentLevel::Summary);
    }

    vm
}

pub fn present_session_turns(
    session: AgentSession,
    offset: usize,
    limit: usize,
    next_cursor: Option<String>,
) -> SessionTurnsViewModel {
    let total_turns = session.turns.len();
    let turns: Vec<_> = session
        .turns
        .into_iter()
        .enumerate()
        .skip(offset)
        .take(limit)
        .map(|(global_idx, turn)| TurnWithIndex {
            turn_index: global_idx,
            turn,
        })
        .collect();

    let mut vm = SessionTurnsViewModel {
        session_id: session.session_id.to_string(),
        start_time: session.start_time,
        end_time: session.end_time,
        stats: session.stats,
        turns,
        _meta: ResponseMeta::from_bytes(0),
    };

    if let Ok(json) = serde_json::to_string(&vm) {
        let bytes = json.len();
        vm._meta =
            ResponseMeta::with_pagination(bytes, next_cursor, vm.turns.len(), Some(total_turns))
                .with_content_level(ContentLevel::Turns);
    }

    vm
}

pub fn present_session_full(
    mut session: AgentSession,
    offset: usize,
    limit: usize,
    next_cursor: Option<String>,
) -> SessionFullViewModel {
    let total_turns = session.turns.len();

    session.turns = session.turns.into_iter().skip(offset).take(limit).collect();

    let mut meta = ResponseMeta::from_bytes(0);
    if let Ok(json) = serde_json::to_string(&session) {
        let bytes = json.len();
        meta = ResponseMeta::with_pagination(
            bytes,
            next_cursor,
            session.turns.len(),
            Some(total_turns),
        )
        .with_content_level(ContentLevel::Full);
    }

    SessionFullViewModel::new(session, meta)
}
