use agtrace_sdk::types::AgentSession;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::mcp::models::common::{ContentLevel, ResponseMeta};

/// Get complete session data with full payloads (50-100 KB per chunk, requires pagination)
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetSessionFullArgs {
    /// Session ID obtained from list_sessions response (use the 'id' field).
    /// Accepts 8-character prefix (e.g., "fb3cff44") or full UUID.
    /// REQUIRED: Cannot be empty.
    pub session_id: String,

    /// Pagination cursor. Use null/"start" for first call, then next_cursor from responses.
    /// REQUIRED for safety: forces explicit pagination awareness.
    #[serde(default)]
    pub cursor: Option<String>,

    /// Maximum number of turns per chunk (default: 5, max: 10).
    /// Kept small to ensure chunks stay under 100 KB.
    #[serde(default)]
    pub limit: Option<usize>,
}

impl GetSessionFullArgs {
    pub fn limit(&self) -> usize {
        self.limit.unwrap_or(5).min(10)
    }

    /// Check if this is the initial request (no cursor or "start")
    pub fn is_initial(&self) -> bool {
        self.cursor.is_none() || self.cursor.as_deref() == Some("start")
    }
}

#[derive(Debug, Serialize)]
pub struct SessionFullViewModel {
    session: AgentSession,
    #[serde(rename = "_meta")]
    meta: ResponseMeta,
}

impl SessionFullViewModel {
    pub fn new(
        mut session: AgentSession,
        offset: usize,
        limit: usize,
        next_cursor: Option<String>,
    ) -> Self {
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

        Self { session, meta }
    }
}
