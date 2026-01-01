use agtrace_sdk::types::AgentSession;
use serde_json::Value;

/// Session full response (detail_level: full)
/// Target size: Unbounded (use with caution)
/// Returns the complete AgentSession without any truncation
pub struct SessionFullResponse {
    session: AgentSession,
}

impl SessionFullResponse {
    pub fn from_session(session: AgentSession) -> Self {
        Self { session }
    }

    pub fn into_value(self) -> Result<Value, String> {
        serde_json::to_value(&self.session).map_err(|e| format!("Serialization error: {}", e))
    }
}
