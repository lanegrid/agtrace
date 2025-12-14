use crate::session::AgentSession;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub event_counts: EventCounts,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventCounts {
    pub total: usize,
    pub user_messages: usize,
    pub assistant_messages: usize,
    pub tool_calls: usize,
    pub reasoning_blocks: usize,
}

pub fn summarize(session: &AgentSession) -> SessionSummary {
    let user_count = session.turns.len();
    let mut assistant_count = 0;
    let mut tool_call_count = 0;
    let mut reasoning_count = 0;
    let mut total_event_count = 0;

    for turn in &session.turns {
        total_event_count += 1;

        for step in &turn.steps {
            if step.message.is_some() {
                assistant_count += 1;
                total_event_count += 1;
            }
            if step.reasoning.is_some() {
                reasoning_count += 1;
                total_event_count += 1;
            }

            tool_call_count += step.tools.len();
            total_event_count += step.tools.len() * 2;

            if step.usage.is_some() {
                total_event_count += 1;
            }
        }
    }

    SessionSummary {
        event_counts: EventCounts {
            total: total_event_count,
            user_messages: user_count,
            assistant_messages: assistant_count,
            tool_calls: tool_call_count,
            reasoning_blocks: reasoning_count,
        },
    }
}
