use agtrace_sdk::types::{AgentSession, AgentTurn, SessionStats};
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::mcp::dto::common::ResponseMeta;

/// Session turns response for get_session_turns tool
/// Target size: 10-30 KB per page (paginated)
/// Returns AgentTurn list directly with minimal transformation
#[derive(Debug, Serialize)]
pub struct SessionTurnsResponse {
    pub session_id: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub stats: SessionStats,
    pub turns: Vec<TurnWithIndex>,
    pub _meta: ResponseMeta,
}

/// Turn with its global index
#[derive(Debug, Serialize)]
pub struct TurnWithIndex {
    /// Global turn index (0-based position in full session, not page-relative)
    pub turn_index: usize,
    #[serde(flatten)]
    pub turn: AgentTurn,
}

impl SessionTurnsResponse {
    #[allow(dead_code)]
    pub fn from_session(session: AgentSession, _include_reasoning: bool) -> Self {
        let turns = session
            .turns
            .into_iter()
            .enumerate()
            .map(|(idx, turn)| TurnWithIndex {
                turn_index: idx,
                turn,
            })
            .collect();

        Self {
            session_id: session.session_id.to_string(),
            start_time: session.start_time,
            end_time: session.end_time,
            stats: session.stats,
            turns,
            _meta: ResponseMeta::from_bytes(0),
        }
    }

    pub fn from_session_paginated(
        session: AgentSession,
        offset: usize,
        limit: usize,
        next_cursor: Option<String>,
    ) -> Self {
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

        let response = Self {
            session_id: session.session_id.to_string(),
            start_time: session.start_time,
            end_time: session.end_time,
            stats: session.stats,
            turns,
            _meta: ResponseMeta::from_bytes(0),
        };

        response.with_metadata(next_cursor, total_turns)
    }

    pub fn with_metadata(mut self, next_cursor: Option<String>, total_turns: usize) -> Self {
        if let Ok(json) = serde_json::to_string(&self) {
            let bytes = json.len();
            self._meta = ResponseMeta::with_pagination(
                bytes,
                next_cursor,
                self.turns.len(),
                Some(total_turns),
            )
            .with_content_level(crate::mcp::dto::common::ContentLevel::Turns);
        }
        self
    }
}
