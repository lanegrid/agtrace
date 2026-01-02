use agtrace_sdk::types::{AgentSession, AgentTurn, SessionStats, TurnStats};
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::mcp::dto::common::truncate_string;

const MAX_SNIPPET_LEN: usize = 200;

/// Session summary response (detail_level: summary)
/// Target size: 5-10 KB
#[derive(Debug, Serialize)]
pub struct SessionSummaryResponse {
    pub session_id: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub stats: SessionStats,
    pub turns: Vec<TurnSummaryDto>,
}

#[derive(Debug, Serialize)]
pub struct TurnSummaryDto {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub user_message: String,
    pub step_count: usize,
    pub stats: TurnStats,
}

impl SessionSummaryResponse {
    pub fn from_session(session: AgentSession) -> Self {
        Self {
            session_id: session.session_id.to_string(),
            start_time: session.start_time,
            end_time: session.end_time,
            stats: session.stats,
            turns: session
                .turns
                .into_iter()
                .map(TurnSummaryDto::from_turn)
                .collect(),
        }
    }
}

impl TurnSummaryDto {
    pub fn from_turn(turn: AgentTurn) -> Self {
        Self {
            id: turn.id.to_string(),
            timestamp: turn.timestamp,
            user_message: truncate_string(&turn.user.content.text, MAX_SNIPPET_LEN),
            step_count: turn.steps.len(),
            stats: turn.stats,
        }
    }
}
