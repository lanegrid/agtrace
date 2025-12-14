use crate::session::AgentSession;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub event_counts: EventCounts,
    pub token_stats: TokenStats,
    pub duration: Option<Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventCounts {
    pub total: usize,
    pub user_messages: usize,
    pub assistant_messages: usize,
    pub tool_calls: usize,
    pub reasoning_blocks: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenStats {
    pub total: u64,
    pub input: u64,
    pub output: u64,
    pub cached: u64,
    pub thinking: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Duration {
    pub minutes: i64,
    pub seconds: i64,
}

pub fn summarize(session: &AgentSession) -> SessionSummary {
    let user_count = session.turns.len();
    let mut assistant_count = 0;
    let mut tool_call_count = 0;
    let mut reasoning_count = 0;
    let mut total_event_count = 0;

    let mut total_input = 0u64;
    let mut total_output = 0u64;
    let mut total_cached = 0u64;
    let mut total_thinking = 0u64;

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

            if let Some(usage) = &step.usage {
                total_input += usage.input_tokens as u64;
                total_output += usage.output_tokens as u64;
                total_event_count += 1;

                if let Some(details) = &usage.details {
                    if let Some(cached) = details.cache_read_input_tokens {
                        total_cached += cached as u64;
                    }
                    if let Some(thinking) = details.reasoning_output_tokens {
                        total_thinking += thinking as u64;
                    }
                }
            }
        }
    }

    let duration = if let Some(end) = session.end_time {
        let duration = end.signed_duration_since(session.start_time);
        Some(Duration {
            minutes: duration.num_minutes(),
            seconds: duration.num_seconds() % 60,
        })
    } else {
        None
    };

    SessionSummary {
        event_counts: EventCounts {
            total: total_event_count,
            user_messages: user_count,
            assistant_messages: assistant_count,
            tool_calls: tool_call_count,
            reasoning_blocks: reasoning_count,
        },
        token_stats: TokenStats {
            total: total_input + total_output,
            input: total_input,
            output: total_output,
            cached: total_cached,
            thinking: total_thinking,
        },
        duration,
    }
}
