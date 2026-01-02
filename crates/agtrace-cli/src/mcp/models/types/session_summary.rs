use agtrace_sdk::types::AgentSession;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::mcp::models::common::{ContentLevel, ResponseMeta};

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
    #[serde(flatten)]
    pub session: AgentSession,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_hash: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,

    pub _meta: ResponseMeta,
}

impl SessionSummaryViewModel {
    pub fn new(
        session: AgentSession,
        metadata: Option<agtrace_sdk::types::SessionMetadata>,
    ) -> Self {
        let (project_hash, provider) = metadata
            .map(|m| (Some(m.project_hash.to_string()), Some(m.provider)))
            .unwrap_or((None, None));

        let mut vm = Self {
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
}
