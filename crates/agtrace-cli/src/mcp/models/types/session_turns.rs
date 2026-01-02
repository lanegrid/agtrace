use agtrace_sdk::types::{AgentSession, SessionStats};
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::mcp::models::common::{ContentLevel, ResponseMeta};

/// Get turn-level summaries with pagination (10-30 KB per page)
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetSessionTurnsArgs {
    /// Session ID obtained from list_sessions response (use the 'id' field).
    /// Accepts 8-character prefix (e.g., "fb3cff44") or full UUID.
    /// REQUIRED: Cannot be empty.
    pub session_id: String,

    /// Pagination cursor from previous response's next_cursor field. Omit for first page.
    #[serde(default)]
    pub cursor: Option<String>,

    /// Maximum number of turns to return per page (default: 10, max: 50)
    #[serde(default)]
    pub limit: Option<usize>,
}

impl GetSessionTurnsArgs {
    pub fn limit(&self) -> usize {
        self.limit.unwrap_or(10).min(50)
    }
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

impl SessionTurnsViewModel {
    pub fn new(
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

        let mut vm = Self {
            session_id: session.session_id.to_string(),
            start_time: session.start_time,
            end_time: session.end_time,
            stats: session.stats,
            turns,
            _meta: ResponseMeta::from_bytes(0),
        };

        if let Ok(json) = serde_json::to_string(&vm) {
            let bytes = json.len();
            vm._meta = ResponseMeta::with_pagination(
                bytes,
                next_cursor,
                vm.turns.len(),
                Some(total_turns),
            )
            .with_content_level(ContentLevel::Turns);
        }

        vm
    }
}

#[derive(Debug, Serialize)]
pub struct TurnWithIndex {
    pub turn_index: usize,
    #[serde(flatten)]
    pub turn: agtrace_sdk::types::AgentTurn,
}
