use agtrace_sdk::types::AgentTurn;

use crate::mcp::models::common::{ContentLevel, ResponseMeta};
use crate::mcp::models::response::TurnStepsViewModel;

pub fn present_turn_steps(
    session_id: String,
    turn_index: usize,
    turn: AgentTurn,
) -> TurnStepsViewModel {
    let mut vm = TurnStepsViewModel {
        session_id,
        turn_index,
        turn,
        _meta: ResponseMeta::from_bytes(0),
    };

    if let Ok(json) = serde_json::to_string(&vm) {
        let bytes = json.len();
        let meta = ResponseMeta::with_pagination(
            bytes,
            None,
            vm.turn.steps.len(),
            Some(vm.turn.steps.len()),
        )
        .with_content_level(ContentLevel::Steps);

        vm._meta = meta;
    }

    vm
}
