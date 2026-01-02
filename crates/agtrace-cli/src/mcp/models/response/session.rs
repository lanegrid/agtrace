use agtrace_sdk::types::{AgentSession, SessionStats};
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::mcp::models::common::ResponseMeta;

#[derive(Debug, Serialize)]
pub struct ListSessionsViewModel {
    pub sessions: Vec<SessionSummaryDto>,
    pub total_in_page: usize,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(transparent)]
pub struct SessionSummaryDto(pub agtrace_sdk::SessionSummary);

#[derive(Debug, Serialize)]
pub struct SessionSummaryViewModel {
    #[serde(flatten)]
    pub session: AgentSession,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_hash: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,

    pub _meta: ResponseMeta,
}

#[derive(Debug, Serialize)]
pub struct SessionTurnsViewModel {
    pub session_id: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub stats: SessionStats,
    pub turns: Vec<TurnWithIndex>,
    pub _meta: ResponseMeta,
}

#[derive(Debug, Serialize)]
pub struct TurnWithIndex {
    pub turn_index: usize,
    #[serde(flatten)]
    pub turn: agtrace_sdk::types::AgentTurn,
}

#[derive(Debug, Serialize)]
pub struct SessionFullViewModel {
    session: AgentSession,
    #[serde(rename = "_meta")]
    meta: ResponseMeta,
}

impl SessionFullViewModel {
    pub fn new(session: AgentSession, meta: ResponseMeta) -> Self {
        Self { session, meta }
    }
}
