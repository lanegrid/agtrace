use agtrace_types::{AgentEventV1, EventType, FileOp};
use chrono::DateTime;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub event_counts: EventCounts,
    pub file_operations: HashMap<FileOp, usize>,
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

pub fn summarize(events: &[AgentEventV1]) -> SessionSummary {
    if events.is_empty() {
        return SessionSummary {
            event_counts: EventCounts {
                total: 0,
                user_messages: 0,
                assistant_messages: 0,
                tool_calls: 0,
                reasoning_blocks: 0,
            },
            file_operations: HashMap::new(),
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
    let mut file_ops = HashMap::new();

    let mut total_input = 0u64;
    let mut total_output = 0u64;
    let mut total_cached = 0u64;
    let mut total_thinking = 0u64;

    for event in events {
        match event.event_type {
            EventType::UserMessage => user_count += 1,
            EventType::AssistantMessage => assistant_count += 1,
            EventType::ToolCall => tool_call_count += 1,
            EventType::Reasoning => reasoning_count += 1,
            _ => {}
        }

        if let Some(file_op) = &event.file_op {
            *file_ops.entry(*file_op).or_insert(0) += 1;
        }

        if let Some(t) = event.tokens_input {
            total_input += t;
        }
        if let Some(t) = event.tokens_output {
            total_output += t;
        }
        if let Some(t) = event.tokens_cached {
            total_cached += t;
        }
        if let Some(t) = event.tokens_thinking {
            total_thinking += t;
        }
    }

    let duration = if let (Some(first), Some(last)) = (events.first(), events.last()) {
        if let (Ok(start), Ok(end)) = (
            DateTime::parse_from_rfc3339(&first.ts),
            DateTime::parse_from_rfc3339(&last.ts),
        ) {
            let duration = end.signed_duration_since(start);
            Some(Duration {
                minutes: duration.num_minutes(),
                seconds: duration.num_seconds() % 60,
            })
        } else {
            None
        }
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
        file_operations: file_ops,
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
