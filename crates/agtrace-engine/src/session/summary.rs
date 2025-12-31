use super::types::AgentSession;
use serde::{Deserialize, Serialize};

/// Statistical summary of session event composition.
///
/// Provides aggregated counts of different event types within a session
/// for analysis and reporting purposes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    /// Breakdown of event counts by type.
    pub event_counts: EventCounts,
}

/// Count of events by type within a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventCounts {
    /// Total number of events in the session.
    pub total: usize,
    /// Number of user input messages.
    pub user_messages: usize,
    /// Number of assistant response messages.
    pub assistant_messages: usize,
    /// Number of tool call invocations.
    pub tool_calls: usize,
    /// Number of reasoning/thinking blocks.
    pub reasoning_blocks: usize,
}

/// Generate statistical summary from a session.
///
/// Counts all events by type to produce an aggregated view of session composition.
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
