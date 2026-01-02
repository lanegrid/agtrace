use agtrace_sdk::types::AgentSession;
use serde_json::Value;

use super::{
    common::DetailLevel,
    response::{
        SessionFullResponse, SessionStepsResponse, SessionSummaryResponse, SessionTurnsResponse,
    },
};

/// Builder for creating session responses with different detail levels
pub struct SessionResponseBuilder {
    session: AgentSession,
    detail_level: DetailLevel,
    include_reasoning: bool,
}

impl SessionResponseBuilder {
    pub fn new(session: AgentSession) -> Self {
        Self {
            session,
            detail_level: DetailLevel::default(),
            include_reasoning: false,
        }
    }

    pub fn detail_level(mut self, level: DetailLevel) -> Self {
        self.detail_level = level;
        self
    }

    pub fn include_reasoning(mut self, include: bool) -> Self {
        self.include_reasoning = include;
        self
    }

    pub fn build(self) -> Result<Value, String> {
        match self.detail_level {
            DetailLevel::Summary => {
                let response = SessionSummaryResponse::from_session(self.session).with_metadata();
                serde_json::to_value(&response).map_err(|e| format!("Serialization error: {}", e))
            }
            DetailLevel::Turns => {
                let response =
                    SessionTurnsResponse::from_session(self.session, self.include_reasoning);
                serde_json::to_value(&response).map_err(|e| format!("Serialization error: {}", e))
            }
            DetailLevel::Steps => {
                let response = SessionStepsResponse::from_session(self.session);
                response.into_value()
            }
            DetailLevel::Full => {
                let response = SessionFullResponse::from_session(self.session);
                response.into_value()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agtrace_sdk::types::{AgentSession, SessionStats};
    use chrono::Utc;
    use uuid::Uuid;

    fn create_test_session() -> AgentSession {
        AgentSession {
            session_id: Uuid::new_v4(),
            start_time: Utc::now(),
            end_time: Some(Utc::now()),
            turns: vec![],
            stats: SessionStats::default(),
        }
    }

    #[test]
    fn test_builder_summary() {
        let session = create_test_session();
        let result = SessionResponseBuilder::new(session)
            .detail_level(DetailLevel::Summary)
            .build();

        assert!(result.is_ok());
    }

    #[test]
    fn test_builder_turns() {
        let session = create_test_session();
        let result = SessionResponseBuilder::new(session)
            .detail_level(DetailLevel::Turns)
            .include_reasoning(true)
            .build();

        assert!(result.is_ok());
    }
}
