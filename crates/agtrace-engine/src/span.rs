use agtrace_types::{AgentEventV1, EventType, ToolStatus};
use serde::{Deserialize, Serialize};

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

/// Build spans from a sequence of events.
/// Each span starts with a UserMessage and continues until the next UserMessage.
pub fn build_spans(events: &[AgentEventV1]) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut current_span = Span::new();
    let mut pending_tools: Vec<PendingTool> = Vec::new();

    for event in events {
        match event.event_type {
            EventType::UserMessage => {
                // Start new span
                if has_content(&current_span) {
                    finalize_span(&mut current_span);
                    spans.push(std::mem::take(&mut current_span));
                }

                current_span.user = Some(Message {
                    ts: event.ts.clone(),
                    role: "user".to_string(),
                    text: event.text.clone().unwrap_or_default(),
                    tokens: None,
                });
            }

            EventType::AssistantMessage => {
                if let Some(text) = &event.text {
                    current_span.assistant.push(Message {
                        ts: event.ts.clone(),
                        role: "assistant".to_string(),
                        text: text.clone(),
                        tokens: extract_token_bundle(event),
                    });
                }
            }

            EventType::Reasoning => {
                if let Some(text) = &event.text {
                    current_span.reasoning.push(text.clone());
                }
            }

            EventType::ToolCall => {
                let tool_name = event.tool_name.clone().unwrap_or_default();
                let input_summary = extract_input_summary(event);

                let tool_index = current_span.tools.len();
                current_span.tools.push(ToolAction {
                    call_id: event.tool_call_id.clone(),
                    ts_call: event.ts.clone(),
                    tool_name,
                    input_summary,
                    ts_result: None,
                    status: None,
                    exit_code: None,
                    latency_ms: None,
                    error_summary: None,
                });

                pending_tools.push(PendingTool {
                    call_id: event.tool_call_id.clone(),
                    tool_index,
                });

                current_span.stats.tool_calls += 1;
            }

            EventType::ToolResult => {
                // Match with pending tool call
                if let Some(call_id) = &event.tool_call_id {
                    if let Some(pos) = pending_tools
                        .iter()
                        .position(|p| p.call_id.as_ref() == Some(call_id))
                    {
                        let pending = pending_tools.remove(pos);
                        if let Some(tool) = current_span.tools.get_mut(pending.tool_index) {
                            tool.ts_result = Some(event.ts.clone());
                            tool.status = event.tool_status;
                            tool.exit_code = event.tool_exit_code;
                            tool.latency_ms = event.tool_latency_ms;

                            if matches!(event.tool_status, Some(ToolStatus::Error))
                                || event.tool_exit_code.is_some_and(|c| c != 0)
                            {
                                current_span.stats.tool_failures += 1;
                                tool.error_summary =
                                    event.text.as_ref().map(|t| truncate_string(t, 100));
                            }
                        }
                    } else {
                        // No matching call_id, try to match with the most recent uncompleted tool
                        if let Some(tool) = current_span
                            .tools
                            .iter_mut()
                            .rev()
                            .find(|t| t.ts_result.is_none())
                        {
                            tool.ts_result = Some(event.ts.clone());
                            tool.status = event.tool_status;
                            tool.exit_code = event.tool_exit_code;
                            tool.latency_ms = event.tool_latency_ms;

                            if matches!(event.tool_status, Some(ToolStatus::Error))
                                || event.tool_exit_code.is_some_and(|c| c != 0)
                            {
                                current_span.stats.tool_failures += 1;
                                tool.error_summary =
                                    event.text.as_ref().map(|t| truncate_string(t, 100));
                            }
                        }
                    }
                }
            }

            EventType::SystemMessage => {
                if let Some(message) = &event.text {
                    current_span.system.push(SystemEvent {
                        ts: event.ts.clone(),
                        message: message.clone(),
                    });
                }
            }

            _ => {}
        }
    }

    // Finalize last span
    if has_content(&current_span) {
        finalize_span(&mut current_span);
        spans.push(current_span);
    }

    spans
}

struct PendingTool {
    call_id: Option<String>,
    tool_index: usize,
}

fn has_content(span: &Span) -> bool {
    span.user.is_some()
        || !span.assistant.is_empty()
        || !span.tools.is_empty()
        || !span.reasoning.is_empty()
        || !span.system.is_empty()
}

fn finalize_span(span: &mut Span) {
    calculate_stats(span);
}

fn calculate_stats(span: &mut Span) {
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

fn extract_token_bundle(event: &AgentEventV1) -> Option<TokenBundle> {
    if event.tokens_input.is_some() || event.tokens_output.is_some() || event.tokens_total.is_some()
    {
        Some(TokenBundle {
            input: event.tokens_input,
            output: event.tokens_output,
            total: event.tokens_total,
            cached: event.tokens_cached,
            thinking: event.tokens_thinking,
        })
    } else {
        None
    }
}

fn extract_input_summary(event: &AgentEventV1) -> String {
    // Try file path first
    if let Some(file_path) = &event.file_path {
        if let Some(filename) = std::path::Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
        {
            return filename.to_string();
        }
    }

    // Try parsing JSON from text field
    if let Some(text) = &event.text {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(text) {
            if let Some(cmd) = json.get("command").and_then(|v| v.as_str()) {
                return truncate_string(cmd, 50);
            }
            if let Some(pattern) = json.get("pattern").and_then(|v| v.as_str()) {
                return format!("\"{}\"", truncate_string(pattern, 30));
            }
        }
    }

    String::new()
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let chars: Vec<char> = s.chars().take(max_len - 3).collect();
        format!("{}...", chars.iter().collect::<String>())
    }
}
