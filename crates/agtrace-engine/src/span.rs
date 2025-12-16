use agtrace_types::v2::{AgentEvent, EventPayload};
use agtrace_types::ToolStatus;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// A Span represents a unit of user-initiated work, starting from a user message
/// and including all assistant responses, tool calls, and reasoning until the next user message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    pub user: Option<Message>,
    pub assistant: Vec<Message>,
    pub tools: Vec<ToolAction>,
    pub reasoning: Vec<String>,
    pub system: Vec<SystemEvent>,
    pub stats: SpanStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub ts: String,
    pub role: String,
    pub text: String,
    pub tokens: Option<TokenBundle>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBundle {
    pub input: Option<u64>,
    pub output: Option<u64>,
    pub total: Option<u64>,
    pub cached: Option<u64>,
    pub thinking: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolAction {
    pub call_id: Option<String>,
    pub ts_call: String,
    pub tool_name: String,
    pub input_summary: String,
    pub ts_result: Option<String>,
    pub status: Option<ToolStatus>,
    pub exit_code: Option<i32>,
    pub latency_ms: Option<u64>,
    pub error_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemEvent {
    pub ts: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpanStats {
    pub tool_calls: usize,
    pub tool_failures: usize,
    pub e2e_ms: Option<u64>,
    pub pre_tool_ms: Option<u64>,
    pub tool_ms: Option<u64>,
    pub post_tool_ms: Option<u64>,
    pub tokens_total: Option<u64>,
}

impl Span {
    pub fn new() -> Self {
        Self {
            user: None,
            assistant: Vec::new(),
            tools: Vec::new(),
            reasoning: Vec::new(),
            system: Vec::new(),
            stats: SpanStats::default(),
        }
    }
}

impl Default for Span {
    fn default() -> Self {
        Self::new()
    }
}

/// Build spans from events.
/// Each span starts with a User event and continues until the next User event.
///
/// Key features:
/// - O(1) tool call/result matching using tool_call_id
/// - TokenUsage as sidecar events (not embedded)
/// - No fallback guessing logic - all references are explicit
pub fn build_spans(events: &[AgentEvent]) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut current_span = Span::new();

    // Map: tool_call event id -> index in current_span.tools
    let mut tool_call_map: HashMap<Uuid, usize> = HashMap::new();

    // Map: generation event id -> accumulated tokens
    let mut token_map: HashMap<Uuid, TokenBundle> = HashMap::new();

    // Map: message event id -> index in current_span.assistant
    let mut message_map: HashMap<Uuid, usize> = HashMap::new();

    for event in events {
        match &event.payload {
            EventPayload::User(payload) => {
                // Start new span
                if has_content(&current_span) {
                    finalize_span(&mut current_span, &token_map, &message_map);
                    spans.push(std::mem::take(&mut current_span));
                    tool_call_map.clear();
                    token_map.clear();
                    message_map.clear();
                }

                current_span.user = Some(Message {
                    ts: event.timestamp.to_rfc3339(),
                    role: "user".to_string(),
                    text: payload.text.clone(),
                    tokens: None,
                });
            }

            EventPayload::Reasoning(payload) => {
                current_span.reasoning.push(payload.text.clone());
            }

            EventPayload::ToolCall(payload) => {
                let tool_index = current_span.tools.len();

                let input_summary = extract_input_summary(payload);

                current_span.tools.push(ToolAction {
                    call_id: payload.provider_call_id.clone(),
                    ts_call: event.timestamp.to_rfc3339(),
                    tool_name: payload.name.clone(),
                    input_summary,
                    ts_result: None,
                    status: None,
                    exit_code: None,
                    latency_ms: None,
                    error_summary: None,
                });

                // Register this tool call for O(1) lookup
                tool_call_map.insert(event.id, tool_index);

                current_span.stats.tool_calls += 1;
            }

            EventPayload::ToolResult(payload) => {
                // O(1) lookup using tool_call_id
                if let Some(&tool_index) = tool_call_map.get(&payload.tool_call_id) {
                    if let Some(tool) = current_span.tools.get_mut(tool_index) {
                        tool.ts_result = Some(event.timestamp.to_rfc3339());

                        if payload.is_error {
                            tool.status = Some(agtrace_types::ToolStatus::Error);
                            tool.error_summary = Some(truncate_string(&payload.output, 100));
                            current_span.stats.tool_failures += 1;
                        } else {
                            tool.status = Some(agtrace_types::ToolStatus::Success);

                            // Try to extract exit code from output
                            tool.exit_code = extract_exit_code(&payload.output);
                            if tool.exit_code.is_some_and(|c| c != 0) {
                                current_span.stats.tool_failures += 1;
                                tool.error_summary = Some(truncate_string(&payload.output, 100));
                            }
                        }

                        // Calculate latency
                        if let Ok(call_dt) = chrono::DateTime::parse_from_rfc3339(&tool.ts_call) {
                            if let Ok(result_dt) =
                                chrono::DateTime::parse_from_rfc3339(&event.timestamp.to_rfc3339())
                            {
                                let duration = result_dt.signed_duration_since(call_dt);
                                tool.latency_ms = Some(duration.num_milliseconds().max(0) as u64);
                            }
                        }
                    }
                }
            }

            EventPayload::Message(payload) => {
                let message_index = current_span.assistant.len();
                current_span.assistant.push(Message {
                    ts: event.timestamp.to_rfc3339(),
                    role: "assistant".to_string(),
                    text: payload.text.clone(),
                    tokens: None, // Will be filled in finalize_span
                });
                message_map.insert(event.id, message_index);
            }

            EventPayload::TokenUsage(payload) => {
                // TokenUsage is a sidecar event
                // parent_id points to the generation event (ToolCall or Message)
                if let Some(parent_id) = event.parent_id {
                    let bundle = token_map.entry(parent_id).or_insert(TokenBundle {
                        input: None,
                        output: None,
                        total: None,
                        cached: None,
                        thinking: None,
                    });

                    // Accumulate tokens (support incremental updates)
                    bundle.input = Some(bundle.input.unwrap_or(0) + payload.input_tokens as u64);
                    bundle.output = Some(bundle.output.unwrap_or(0) + payload.output_tokens as u64);
                    bundle.total = Some(bundle.total.unwrap_or(0) + payload.total_tokens as u64);

                    if let Some(details) = &payload.details {
                        if let Some(cached) = details.cache_read_input_tokens {
                            bundle.cached = Some(bundle.cached.unwrap_or(0) + cached as u64);
                        }
                        if let Some(thinking) = details.reasoning_output_tokens {
                            bundle.thinking = Some(bundle.thinking.unwrap_or(0) + thinking as u64);
                        }
                    }
                }
            }

            EventPayload::Notification(_) => {
                // Skip notifications - they are not part of span structure
                // Used for watch display only
            }
        }
    }

    // Finalize last span
    if has_content(&current_span) {
        finalize_span(&mut current_span, &token_map, &message_map);
        spans.push(current_span);
    }

    spans
}

fn has_content(span: &Span) -> bool {
    span.user.is_some()
        || !span.assistant.is_empty()
        || !span.tools.is_empty()
        || !span.reasoning.is_empty()
        || !span.system.is_empty()
}

fn finalize_span(
    span: &mut Span,
    token_map: &HashMap<Uuid, TokenBundle>,
    message_map: &HashMap<Uuid, usize>,
) {
    // Attach tokens to messages
    for (msg_id, msg_idx) in message_map {
        if let Some(tokens) = token_map.get(msg_id) {
            if let Some(msg) = span.assistant.get_mut(*msg_idx) {
                msg.tokens = Some(tokens.clone());
            }
        }
    }

    calculate_stats(span, token_map);
}

fn calculate_stats(span: &mut Span, _token_map: &HashMap<Uuid, TokenBundle>) {
    use chrono::DateTime;

    // Calculate tokens_total from assistant messages
    let mut total_tokens = 0u64;
    for msg in &span.assistant {
        if let Some(tokens) = &msg.tokens {
            if let Some(t) = tokens.total {
                total_tokens += t;
            }
        }
    }
    if total_tokens > 0 {
        span.stats.tokens_total = Some(total_tokens);
    }

    // Calculate timing stats
    let user_ts = span.user.as_ref().map(|u| &u.ts);
    let first_tool_ts = span.tools.first().map(|t| &t.ts_call);
    let last_tool_result_ts = span.tools.iter().rev().find_map(|t| t.ts_result.as_ref());
    let last_event_ts = span
        .assistant
        .last()
        .map(|a| &a.ts)
        .or(last_tool_result_ts)
        .or(span.system.last().map(|s| &s.ts));

    // pre_tool_ms = first_tool_call_ts - user_ts
    if let (Some(user_ts), Some(first_tool_ts)) = (user_ts, first_tool_ts) {
        if let (Ok(user_dt), Ok(tool_dt)) = (
            DateTime::parse_from_rfc3339(user_ts),
            DateTime::parse_from_rfc3339(first_tool_ts),
        ) {
            let duration = tool_dt.signed_duration_since(user_dt);
            span.stats.pre_tool_ms = Some(duration.num_milliseconds().max(0) as u64);
        }
    }

    // tool_ms = last_tool_result_ts - first_tool_call_ts
    if let (Some(first_tool_ts), Some(last_result_ts)) = (first_tool_ts, last_tool_result_ts) {
        if let (Ok(first_dt), Ok(last_dt)) = (
            DateTime::parse_from_rfc3339(first_tool_ts),
            DateTime::parse_from_rfc3339(last_result_ts),
        ) {
            let duration = last_dt.signed_duration_since(first_dt);
            span.stats.tool_ms = Some(duration.num_milliseconds().max(0) as u64);
        }
    }

    // post_tool_ms = last_event_ts - last_tool_result_ts
    if let (Some(last_result_ts), Some(last_event_ts)) = (last_tool_result_ts, last_event_ts) {
        if let (Ok(result_dt), Ok(event_dt)) = (
            DateTime::parse_from_rfc3339(last_result_ts),
            DateTime::parse_from_rfc3339(last_event_ts),
        ) {
            let duration = event_dt.signed_duration_since(result_dt);
            span.stats.post_tool_ms = Some(duration.num_milliseconds().max(0) as u64);
        }
    }

    // e2e_ms = last_event_ts - user_ts
    if let (Some(user_ts), Some(last_event_ts)) = (user_ts, last_event_ts) {
        if let (Ok(user_dt), Ok(event_dt)) = (
            DateTime::parse_from_rfc3339(user_ts),
            DateTime::parse_from_rfc3339(last_event_ts),
        ) {
            let duration = event_dt.signed_duration_since(user_dt);
            span.stats.e2e_ms = Some(duration.num_milliseconds().max(0) as u64);
        }
    }
}

fn extract_input_summary(payload: &agtrace_types::v2::ToolCallPayload) -> String {
    // Try to extract meaningful summary from arguments
    if let Some(file_path) = payload.arguments.get("file_path").and_then(|v| v.as_str()) {
        if let Some(filename) = std::path::Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
        {
            return filename.to_string();
        }
    }

    if let Some(cmd) = payload.arguments.get("command").and_then(|v| v.as_str()) {
        return truncate_string(cmd, 50);
    }

    if let Some(pattern) = payload.arguments.get("pattern").and_then(|v| v.as_str()) {
        return format!("\"{}\"", truncate_string(pattern, 30));
    }

    String::new()
}

fn extract_exit_code(output: &str) -> Option<i32> {
    // Try to extract exit code from common patterns
    // e.g., "exit code: 1" or "Exit code 1"
    let re = regex::Regex::new(r"(?i)exit\s+code:?\s*(\d+)").ok()?;
    re.captures(output)
        .and_then(|cap| cap.get(1))
        .and_then(|m| m.as_str().parse::<i32>().ok())
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let chars: Vec<char> = s.chars().take(max_len - 3).collect();
        format!("{}...", chars.iter().collect::<String>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agtrace_types::v2::*;
    use chrono::Utc;

    #[test]
    fn test_build_spans_basic() {
        let trace_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let tool_call_id = Uuid::new_v4();
        let tool_result_id = Uuid::new_v4();
        let message_id = Uuid::new_v4();

        let events = vec![
            AgentEvent {
                id: user_id,
                trace_id,
                parent_id: None,
                timestamp: Utc::now(),
                payload: EventPayload::User(UserPayload {
                    text: "Calculate 1+1".to_string(),
                }),
                metadata: None,
            },
            AgentEvent {
                id: tool_call_id,
                trace_id,
                parent_id: Some(user_id),
                timestamp: Utc::now(),
                payload: EventPayload::ToolCall(ToolCallPayload {
                    name: "python".to_string(),
                    arguments: serde_json::json!({"command": "print(1+1)"}),
                    provider_call_id: Some("call_123".to_string()),
                }),
                metadata: None,
            },
            AgentEvent {
                id: tool_result_id,
                trace_id,
                parent_id: Some(tool_call_id),
                timestamp: Utc::now(),
                payload: EventPayload::ToolResult(ToolResultPayload {
                    output: "2".to_string(),
                    tool_call_id,
                    is_error: false,
                }),
                metadata: None,
            },
            AgentEvent {
                id: message_id,
                trace_id,
                parent_id: Some(tool_result_id),
                timestamp: Utc::now(),
                payload: EventPayload::Message(MessagePayload {
                    text: "The answer is 2".to_string(),
                }),
                metadata: None,
            },
        ];

        let spans = build_spans(&events);
        assert_eq!(spans.len(), 1);

        let span = &spans[0];
        assert!(span.user.is_some());
        assert_eq!(span.tools.len(), 1);
        assert_eq!(span.assistant.len(), 1);

        let tool = &span.tools[0];
        assert_eq!(tool.tool_name, "python");
        assert!(tool.ts_result.is_some());
        assert_eq!(tool.status, Some(agtrace_types::ToolStatus::Success));
    }

    #[test]
    fn test_tool_call_result_matching() {
        let trace_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let tool1_id = Uuid::new_v4();
        let tool2_id = Uuid::new_v4();
        let result1_id = Uuid::new_v4();
        let result2_id = Uuid::new_v4();

        let events = vec![
            AgentEvent {
                id: user_id,
                trace_id,
                parent_id: None,
                timestamp: Utc::now(),
                payload: EventPayload::User(UserPayload {
                    text: "test".to_string(),
                }),
                metadata: None,
            },
            AgentEvent {
                id: tool1_id,
                trace_id,
                parent_id: Some(user_id),
                timestamp: Utc::now(),
                payload: EventPayload::ToolCall(ToolCallPayload {
                    name: "bash".to_string(),
                    arguments: serde_json::json!({"command": "ls"}),
                    provider_call_id: None,
                }),
                metadata: None,
            },
            AgentEvent {
                id: tool2_id,
                trace_id,
                parent_id: Some(tool1_id),
                timestamp: Utc::now(),
                payload: EventPayload::ToolCall(ToolCallPayload {
                    name: "grep".to_string(),
                    arguments: serde_json::json!({"pattern": "test"}),
                    provider_call_id: None,
                }),
                metadata: None,
            },
            // Results arrive in reverse order
            AgentEvent {
                id: result2_id,
                trace_id,
                parent_id: Some(tool2_id),
                timestamp: Utc::now(),
                payload: EventPayload::ToolResult(ToolResultPayload {
                    output: "match found".to_string(),
                    tool_call_id: tool2_id,
                    is_error: false,
                }),
                metadata: None,
            },
            AgentEvent {
                id: result1_id,
                trace_id,
                parent_id: Some(result2_id),
                timestamp: Utc::now(),
                payload: EventPayload::ToolResult(ToolResultPayload {
                    output: "file1.txt\nfile2.txt".to_string(),
                    tool_call_id: tool1_id,
                    is_error: false,
                }),
                metadata: None,
            },
        ];

        let spans = build_spans(&events);
        assert_eq!(spans.len(), 1);

        let span = &spans[0];
        assert_eq!(span.tools.len(), 2);

        // Both tools should have results
        assert!(span.tools[0].ts_result.is_some());
        assert!(span.tools[1].ts_result.is_some());
    }

    #[test]
    fn test_token_usage_sidecar() {
        let trace_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let message_id = Uuid::new_v4();
        let token_id = Uuid::new_v4();

        let events = vec![
            AgentEvent {
                id: user_id,
                trace_id,
                parent_id: None,
                timestamp: Utc::now(),
                payload: EventPayload::User(UserPayload {
                    text: "hello".to_string(),
                }),
                metadata: None,
            },
            AgentEvent {
                id: message_id,
                trace_id,
                parent_id: Some(user_id),
                timestamp: Utc::now(),
                payload: EventPayload::Message(MessagePayload {
                    text: "hi".to_string(),
                }),
                metadata: None,
            },
            AgentEvent {
                id: token_id,
                trace_id,
                parent_id: Some(message_id),
                timestamp: Utc::now(),
                payload: EventPayload::TokenUsage(TokenUsagePayload {
                    input_tokens: 100,
                    output_tokens: 50,
                    total_tokens: 150,
                    details: None,
                }),
                metadata: None,
            },
        ];

        let spans = build_spans(&events);
        assert_eq!(spans.len(), 1);

        let span = &spans[0];
        assert_eq!(span.assistant.len(), 1);

        let msg = &span.assistant[0];
        assert!(msg.tokens.is_some());
        assert_eq!(msg.tokens.as_ref().unwrap().total, Some(150));
    }
}
