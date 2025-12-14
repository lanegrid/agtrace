use agtrace_types::v2::{AgentEvent, EventPayload};
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

pub fn summarize(events: &[AgentEvent]) -> SessionSummary {
    if events.is_empty() {
        return SessionSummary {
            event_counts: EventCounts {
                total: 0,
                user_messages: 0,
                assistant_messages: 0,
                tool_calls: 0,
                reasoning_blocks: 0,
            },
            token_stats: TokenStats {
                total: 0,
                input: 0,
                output: 0,
                cached: 0,
                thinking: 0,
            },
            duration: None,
        };
    }

    let mut user_count = 0;
    let mut assistant_count = 0;
    let mut tool_call_count = 0;
    let mut reasoning_count = 0;

    let mut total_input = 0u64;
    let mut total_output = 0u64;
    let mut total_cached = 0u64;
    let mut total_thinking = 0u64;

    for event in events {
        match &event.payload {
            EventPayload::User(_) => user_count += 1,
            EventPayload::Message(_) => assistant_count += 1,
            EventPayload::ToolCall(_) => tool_call_count += 1,
            EventPayload::Reasoning(_) => reasoning_count += 1,
            EventPayload::TokenUsage(usage) => {
                total_input += usage.input_tokens as u64;
                total_output += usage.output_tokens as u64;

                if let Some(details) = &usage.details {
                    if let Some(cached) = details.cache_read_input_tokens {
                        total_cached += cached as u64;
                    }
                    if let Some(thinking) = details.reasoning_output_tokens {
                        total_thinking += thinking as u64;
                    }
                }
            }
            _ => {}
        }
    }

    let duration = if let (Some(first), Some(last)) = (events.first(), events.last()) {
        let duration = last.timestamp.signed_duration_since(first.timestamp);
        Some(Duration {
            minutes: duration.num_minutes(),
            seconds: duration.num_seconds() % 60,
        })
    } else {
        None
    };

    SessionSummary {
        event_counts: EventCounts {
            total: events.len(),
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
