use agtrace_sdk::types::AgentSession;
use serde::Serialize;
use serde_json::Value;

use crate::mcp::dto::common::ResponseMeta;

/// Session full response for get_session_full tool
/// Target size: 50-100 KB per chunk (paginated)
/// Returns complete session data with full payloads (not truncated)
#[derive(Debug, Serialize)]
pub struct SessionFullResponse {
    session: AgentSession,
    #[serde(rename = "_meta")]
    meta: ResponseMeta,
}

impl SessionFullResponse {
    #[allow(dead_code)]
    pub fn from_session(session: AgentSession) -> Self {
        Self {
            session,
            meta: ResponseMeta::from_bytes(0),
        }
    }

    pub fn from_session_paginated(
        mut session: AgentSession,
        offset: usize,
        limit: usize,
        next_cursor: Option<String>,
    ) -> Self {
        let total_turns = session.turns.len();

        // Paginate turns
        session.turns = session.turns.into_iter().skip(offset).take(limit).collect();

        let mut response = Self {
            session,
            meta: ResponseMeta::from_bytes(0),
        };

        response.meta = if let Ok(json) = serde_json::to_string(&response) {
            let bytes = json.len();
            ResponseMeta::with_pagination(
                bytes,
                next_cursor,
                response.session.turns.len(),
                Some(total_turns),
            )
            .with_content_level(crate::mcp::dto::common::ContentLevel::Full)
        } else {
            ResponseMeta::from_bytes(0)
        };

        response
    }

    pub fn into_value(self) -> Result<Value, String> {
        serde_json::to_value(&self).map_err(|e| format!("Serialization error: {}", e))
    }
}
