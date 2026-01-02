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
        let mut session_value = serde_json::to_value(&self.session)
            .map_err(|e| format!("Serialization error: {}", e))?;

        if let Some(obj) = session_value.as_object_mut() {
            obj.insert(
                "hint".to_string(),
                Value::String(
                    "Response may be large. Use search_events(pattern) to find specific events, or detail_level='steps' for truncated payloads"
                        .to_string(),
                ),
            );
        }

        Ok(session_value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agtrace_sdk::types::SessionStats;
    use chrono::Utc;
    use uuid::Uuid;

    #[test]
    fn test_full_response_includes_hint() {
        let session = AgentSession {
            session_id: Uuid::new_v4(),
            start_time: Utc::now(),
            end_time: Some(Utc::now()),
            turns: vec![],
            stats: SessionStats::default(),
        };

        let response = SessionFullResponse::from_session(session);
        let value = response.into_value().unwrap();

        let hint = value.get("hint").and_then(|h| h.as_str());
        assert!(hint.is_some(), "Hint should be present");
        assert!(
            hint.unwrap().contains("search_events"),
            "Hint should mention search_events"
        );
        assert!(
            hint.unwrap().contains("detail_level='steps'"),
            "Hint should suggest steps level"
        );
    }
}
