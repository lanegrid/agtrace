use agtrace_sdk::types::{AgentSession, SessionStats};
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::mcp::models::common::truncate_string;

/// Get lightweight session overview (≤5 KB, always single-page)
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetSessionSummaryArgs {
    /// Session ID obtained from list_sessions response (use the 'id' field).
    /// Accepts 8-character prefix (e.g., "fb3cff44") or full UUID.
    /// REQUIRED: Cannot be empty.
    pub session_id: String,
}

#[derive(Debug, Serialize)]
pub struct SessionSummaryViewModel {
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub stats: SessionStats,
    /// Truncated preview of the initial user prompt to identify context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_prompt: Option<String>,
    /// Truncated preview of the final result (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_result: Option<String>,
}

impl SessionSummaryViewModel {
    pub fn new(
        session: AgentSession,
        metadata: Option<agtrace_sdk::types::SessionMetadata>,
    ) -> Self {
        let (project_hash, provider) = metadata
            .map(|m| (Some(m.project_hash.to_string()), Some(m.provider)))
            .unwrap_or((None, None));

        // Extract initial user input preview from first turn
        let initial_prompt = session
            .turns
            .first()
            .map(|t| truncate_string(&t.user.content.text, 200));

        // Extract final message preview from last turn's last step
        let final_result = session
            .turns
            .last()
            .and_then(|t| t.steps.last())
            .and_then(|s| s.message.as_ref())
            .map(|m| truncate_string(&m.content.text, 200));

        Self {
            session_id: session.session_id.to_string(),
            project_hash,
            provider,
            start_time: session.start_time,
            end_time: session.end_time,
            stats: session.stats,
            initial_prompt,
            final_result,
        }
    }
}
