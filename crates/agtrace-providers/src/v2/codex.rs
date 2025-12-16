use agtrace_types::v2::{self, *};
use chrono::DateTime;
use regex::Regex;
use std::sync::LazyLock;
use uuid::Uuid;

use super::builder::EventBuilder;
use crate::codex::schema;
use crate::codex::schema::CodexRecord;

/// Regex for extracting exit codes from Codex output
/// Example: "Exit Code: 0" or similar patterns
static EXIT_CODE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"Exit Code:\s*(\d+)").unwrap());

/// Normalize Codex session records to v2 events
/// Handles async token notifications, JSON string parsing, and exit code extraction
pub fn normalize_codex_session_v2(records: Vec<CodexRecord>, session_id: &str) -> Vec<AgentEvent> {
    // Create trace_id from session_id (deterministic)
    let trace_id = Uuid::new_v5(&Uuid::NAMESPACE_OID, session_id.as_bytes());
    let mut builder = EventBuilder::new(trace_id);
    let mut events = Vec::new();

    // Track last generation event for attaching TokenUsage (future use)
    let mut _last_generation_event_id: Option<Uuid> = None;

    // Track last seen token usage to deduplicate
    // Codex sends duplicate token_count events with same last_token_usage values
    let mut last_seen_token_usage: Option<(i32, i32, i32)> = None;

    for record in records {
        match record {
            CodexRecord::SessionMeta(_meta) => {
                // SessionMeta doesn't generate events in v2
                // Metadata is preserved in raw field if needed
            }

            CodexRecord::EventMsg(event_msg) => {
                let timestamp = parse_timestamp(&event_msg.timestamp);
                let raw_value = serde_json::to_value(&event_msg).ok();

                match &event_msg.payload {
                    // Skip user_message, agent_message, agent_reasoning
                    // These are duplicated in ResponseItem with richer data (encrypted_content, etc.)
                    schema::EventMsgPayload::UserMessage(_) => {
                        // Skip: duplicated in ResponseItem::Message(user)
                    }

                    schema::EventMsgPayload::AgentMessage(_) => {
                        // Skip: duplicated in ResponseItem::Message(assistant)
                    }

                    schema::EventMsgPayload::AgentReasoning(_) => {
                        // Skip: duplicated in ResponseItem::Reasoning
                    }

                    schema::EventMsgPayload::TokenCount(token_count) => {
                        // TokenUsage sidecar event
                        // IMPORTANT: Keep this - token_count only exists in event_msg, not in response_item
                        if let Some(info) = &token_count.info {
                            let usage = &info.last_token_usage;
                            let usage_triple = (
                                usage.input_tokens as i32,
                                usage.output_tokens as i32,
                                usage.total_tokens as i32,
                            );

                            // Deduplicate: Codex often sends duplicate token_count with same last_token_usage
                            if last_seen_token_usage == Some(usage_triple) {
                                // Skip duplicate
                                continue;
                            }
                            last_seen_token_usage = Some(usage_triple);

                            let event = builder.create_event(
                                timestamp,
                                EventPayload::TokenUsage(TokenUsagePayload {
                                    input_tokens: usage.input_tokens as i32,
                                    output_tokens: usage.output_tokens as i32,
                                    total_tokens: usage.total_tokens as i32,
                                    details: Some(TokenUsageDetails {
                                        cache_creation_input_tokens: None, // Codex doesn't track cache creation separately
                                        cache_read_input_tokens: Some(
                                            usage.cached_input_tokens as i32,
                                        ),
                                        reasoning_output_tokens: Some(
                                            usage.reasoning_output_tokens as i32,
                                        ),
                                        audio_input_tokens: None,
                                        audio_output_tokens: None,
                                    }),
                                }),
                                raw_value.clone(),
                            );
                            events.push(event);
                        }
                    }

                    schema::EventMsgPayload::Unknown => {
                        // Skip unknown event types
                    }
                }
            }

            CodexRecord::ResponseItem(response_item) => {
                let timestamp = parse_timestamp(&response_item.timestamp);

                match &response_item.payload {
                    schema::ResponseItemPayload::Message(message) => {
                        // Extract text from content blocks
                        let text = extract_message_text(&message.content);

                        let payload = if message.role == "user" {
                            EventPayload::User(UserPayload { text })
                        } else {
                            EventPayload::Message(v2::MessagePayload { text })
                        };

                        let event = builder.create_event(
                            timestamp,
                            payload,
                            serde_json::to_value(&response_item).ok(),
                        );

                        if message.role == "assistant" {
                            _last_generation_event_id = Some(event.id);
                        }

                        events.push(event);
                    }

                    schema::ResponseItemPayload::Reasoning(reasoning) => {
                        // Extract text from summary blocks
                        let text = extract_reasoning_text(reasoning);

                        let event = builder.create_event(
                            timestamp,
                            EventPayload::Reasoning(v2::ReasoningPayload { text }),
                            serde_json::to_value(&response_item).ok(),
                        );
                        events.push(event);
                    }

                    schema::ResponseItemPayload::FunctionCall(func_call) => {
                        // Parse JSON string arguments to Value
                        let arguments = parse_json_arguments(&func_call.arguments);

                        let event = builder.create_event(
                            timestamp,
                            EventPayload::ToolCall(ToolCallPayload {
                                name: func_call.name.clone(),
                                arguments,
                                provider_call_id: Some(func_call.call_id.clone()),
                            }),
                            serde_json::to_value(&response_item).ok(),
                        );

                        // Register tool call mapping
                        builder.register_tool_call(func_call.call_id.clone(), event.id);
                        _last_generation_event_id = Some(event.id);
                        events.push(event);
                    }

                    schema::ResponseItemPayload::FunctionCallOutput(output) => {
                        // Extract exit code from output text
                        let exit_code = extract_exit_code(&output.output);

                        if let Some(tool_call_id) = builder.get_tool_call_uuid(&output.call_id) {
                            let event = builder.create_event(
                                timestamp,
                                EventPayload::ToolResult(ToolResultPayload {
                                    output: output.output.clone(),
                                    tool_call_id,
                                    is_error: exit_code.map(|code| code != 0).unwrap_or(false),
                                }),
                                serde_json::to_value(&response_item).ok(),
                            );
                            events.push(event);
                        }
                    }

                    schema::ResponseItemPayload::CustomToolCall(tool_call) => {
                        // Parse JSON string input to Value
                        let arguments = parse_json_arguments(&tool_call.input);

                        let event = builder.create_event(
                            timestamp,
                            EventPayload::ToolCall(ToolCallPayload {
                                name: tool_call.name.clone(),
                                arguments,
                                provider_call_id: Some(tool_call.call_id.clone()),
                            }),
                            serde_json::to_value(&response_item).ok(),
                        );

                        builder.register_tool_call(tool_call.call_id.clone(), event.id);
                        _last_generation_event_id = Some(event.id);
                        events.push(event);
                    }

                    schema::ResponseItemPayload::CustomToolCallOutput(output) => {
                        let exit_code = extract_exit_code(&output.output);

                        if let Some(tool_call_id) = builder.get_tool_call_uuid(&output.call_id) {
                            let event = builder.create_event(
                                timestamp,
                                EventPayload::ToolResult(ToolResultPayload {
                                    output: output.output.clone(),
                                    tool_call_id,
                                    is_error: exit_code.map(|code| code != 0).unwrap_or(false),
                                }),
                                serde_json::to_value(&response_item).ok(),
                            );
                            events.push(event);
                        }
                    }

                    schema::ResponseItemPayload::GhostSnapshot(_snapshot) => {
                        // Skip ghost snapshots for now (file system events)
                    }

                    schema::ResponseItemPayload::Unknown => {
                        // Skip unknown payload types
                    }
                }
            }

            CodexRecord::TurnContext(_turn_context) => {
                // TurnContext doesn't generate events in v2
                // Context metadata is preserved in raw field if needed
            }

            CodexRecord::Unknown => {
                // Skip unknown record types
            }
        }
    }

    events
}

/// Extract text from message content blocks
fn extract_message_text(content: &[schema::MessageContent]) -> String {
    content
        .iter()
        .filter_map(|c| match c {
            schema::MessageContent::InputText { text } => Some(text.as_str()),
            schema::MessageContent::OutputText { text } => Some(text.as_str()),
            schema::MessageContent::Unknown => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Extract text from reasoning summary blocks
fn extract_reasoning_text(reasoning: &schema::ReasoningPayload) -> String {
    let summary_text = reasoning
        .summary
        .iter()
        .filter_map(|s| match s {
            schema::SummaryText::SummaryText { text } => Some(text.as_str()),
            schema::SummaryText::Unknown => None,
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Prefer content over summary if available
    reasoning
        .content
        .as_ref()
        .unwrap_or(&summary_text)
        .to_string()
}

/// Parse JSON string arguments to serde_json::Value
/// If parsing fails, wrap the string in a JSON object
fn parse_json_arguments(args: &str) -> serde_json::Value {
    serde_json::from_str(args).unwrap_or_else(|_| {
        // If not valid JSON, wrap in object
        serde_json::json!({ "raw": args })
    })
}

/// Extract exit code from output text using regex
fn extract_exit_code(output: &str) -> Option<i32> {
    EXIT_CODE_REGEX
        .captures(output)
        .and_then(|cap| cap.get(1))
        .and_then(|m| m.as_str().parse().ok())
}

/// Parse Codex timestamp to DateTime<Utc>
fn parse_timestamp(ts: &str) -> DateTime<chrono::Utc> {
    DateTime::parse_from_rfc3339(ts)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_arguments() {
        // Valid JSON object
        let valid = r#"{"command": "ls -la"}"#;
        let result = parse_json_arguments(valid);
        assert_eq!(result["command"], "ls -la");

        // Valid JSON array
        let array = r#"["arg1", "arg2"]"#;
        let result = parse_json_arguments(array);
        assert!(result.is_array());

        // Invalid JSON - should wrap in object
        let invalid = "not json";
        let result = parse_json_arguments(invalid);
        assert_eq!(result["raw"], "not json");
    }

    #[test]
    fn test_extract_exit_code() {
        assert_eq!(extract_exit_code("Exit Code: 0"), Some(0));
        assert_eq!(extract_exit_code("Exit Code: 127"), Some(127));
        assert_eq!(extract_exit_code("Some output\nExit Code: 1\n"), Some(1));
        assert_eq!(extract_exit_code("No exit code here"), None);
    }

    #[test]
    fn test_extract_message_text() {
        let content = vec![
            schema::MessageContent::InputText {
                text: "Hello".to_string(),
            },
            schema::MessageContent::OutputText {
                text: "World".to_string(),
            },
        ];
        assert_eq!(extract_message_text(&content), "Hello\nWorld");
    }
}
