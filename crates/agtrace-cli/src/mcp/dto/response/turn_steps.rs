use agtrace_sdk::types::AgentTurn;
use serde::Serialize;

use crate::mcp::dto::common::ResponseMeta;

/// Turn steps response for get_turn_steps tool
/// Target size: 20-50 KB (paginated if needed)
/// Returns AgentTurn directly with minimal transformation
#[derive(Debug, Serialize)]
pub struct TurnStepsResponse {
    pub session_id: String,
    pub turn_index: usize,
    pub turn: AgentTurn,
    pub _meta: ResponseMeta,
}

impl TurnStepsResponse {
    pub fn from_turn(
        session_id: String,
        turn_index: usize,
        turn: AgentTurn,
        _include_reasoning: bool,
        _include_tools: bool,
        _include_message: bool,
    ) -> Self {
        let response = Self {
            session_id,
            turn_index,
            turn,
            _meta: ResponseMeta::from_bytes(0),
        };

        response.with_metadata()
    }

    pub fn with_metadata(mut self) -> Self {
        if let Ok(json) = serde_json::to_string(&self) {
            let bytes = json.len();
            let meta = ResponseMeta::with_pagination(
                bytes,
                None,
                self.turn.steps.len(),
                Some(self.turn.steps.len()),
            )
            .with_content_level(crate::mcp::dto::common::ContentLevel::Steps);

            self._meta = meta;
        }
        self
    }
}
