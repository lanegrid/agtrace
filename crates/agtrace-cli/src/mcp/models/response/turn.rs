use agtrace_sdk::types::AgentTurn;
use serde::Serialize;

use crate::mcp::models::common::ResponseMeta;

#[derive(Debug, Serialize)]
pub struct TurnStepsViewModel {
    pub session_id: String,
    pub turn_index: usize,
    pub turn: AgentTurn,
    pub _meta: ResponseMeta,
}
