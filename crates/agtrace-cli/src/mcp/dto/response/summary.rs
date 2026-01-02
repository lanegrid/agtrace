use agtrace_sdk::types::{AgentSession, AgentTurn, SessionStats, TurnStats};
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::mcp::dto::common::{ResponseMeta, truncate_string};

const MAX_SNIPPET_LEN: usize = 200;

/// Session summary response for get_session_summary tool
/// Target size: ≤5 KB (guaranteed single-page)
#[derive(Debug, Serialize)]
pub struct SessionSummaryResponse {
    pub session_id: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub stats: SessionStats,
    pub snippet: String,

    /// Project identifier (optional, not available for standalone sessions)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_hash: Option<String>,

    /// Provider name (optional, not available for standalone sessions)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,

    pub _meta: ResponseMeta,
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
    pub fn from_session(
        session: AgentSession,
        metadata: Option<agtrace_sdk::types::SessionMetadata>,
    ) -> Self {
        // Extract first user message as snippet
        let snippet = session
            .turns
            .first()
            .map(|turn| truncate_string(&turn.user.content.text, MAX_SNIPPET_LEN))
            .unwrap_or_else(|| "(no turns)".to_string());

        let (project_hash, provider) = metadata
            .map(|m| (Some(m.project_hash.to_string()), Some(m.provider)))
            .unwrap_or((None, None));

        let response = Self {
            session_id: session.session_id.to_string(),
            start_time: session.start_time,
            end_time: session.end_time,
            stats: session.stats,
            snippet,
            project_hash,
            provider,
            _meta: ResponseMeta::from_bytes(0), // Placeholder, calculated after serialization
        };

        response
    }

    /// Calculate and set metadata after serialization
    pub fn with_metadata(mut self) -> Self {
        // Serialize to calculate size
        if let Ok(json) = serde_json::to_string(&self) {
            let bytes = json.len();
            self._meta = ResponseMeta::from_bytes(bytes);
        }
        self
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
