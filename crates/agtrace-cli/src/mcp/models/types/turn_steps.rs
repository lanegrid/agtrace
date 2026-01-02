use agtrace_sdk::types::AgentTurn;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Get detailed steps for a specific turn (20-50 KB)
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetTurnStepsArgs {
    /// Session ID obtained from list_sessions or get_session_turns response.
    /// Accepts 8-character prefix (e.g., "fb3cff44") or full UUID.
    /// REQUIRED: Cannot be empty.
    pub session_id: String,

    /// Zero-based turn index (obtained from get_session_turns response).
    /// REQUIRED: Must be valid (0 to turn_count - 1).
    pub turn_index: usize,
}

#[derive(Debug, Serialize)]
pub struct TurnStepsViewModel {
    pub session_id: String,
    pub turn_index: usize,
    pub turn: AgentTurn,
}

impl TurnStepsViewModel {
    pub fn new(session_id: String, turn_index: usize, turn: AgentTurn) -> Self {
        Self {
            session_id,
            turn_index,
            turn,
        }
    }
}
