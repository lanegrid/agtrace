use agtrace_sdk::types::AgentSession;
use serde::Serialize;

use crate::mcp::dto::common::ResponseMeta;

/// Session summary response for get_session_summary tool
/// Target size: ≤5 KB (guaranteed single-page)
/// Returns AgentSession directly with metadata
#[derive(Debug, Serialize)]
pub struct SessionSummaryResponse {
    #[serde(flatten)]
    pub session: AgentSession,

    /// Project identifier (optional, not available for standalone sessions)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_hash: Option<String>,

    /// Provider name (optional, not available for standalone sessions)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,

    pub _meta: ResponseMeta,
}

impl SessionSummaryResponse {
    pub fn from_session(
        session: AgentSession,
        metadata: Option<agtrace_sdk::types::SessionMetadata>,
    ) -> Self {
        let (project_hash, provider) = metadata
            .map(|m| (Some(m.project_hash.to_string()), Some(m.provider)))
            .unwrap_or((None, None));

        Self {
            session,
            project_hash,
            provider,
            _meta: ResponseMeta::from_bytes(0),
        }
    }

    pub fn with_metadata(mut self) -> Self {
        if let Ok(json) = serde_json::to_string(&self) {
            let bytes = json.len();
            self._meta = ResponseMeta::from_bytes(bytes)
                .with_content_level(crate::mcp::dto::common::ContentLevel::Summary);
        }
        self
    }
}
