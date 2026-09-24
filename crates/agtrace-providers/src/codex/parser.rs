use agtrace_types::*;
use chrono::{DateTime, Utc};
use regex::Regex;
use std::sync::LazyLock;
use uuid::Uuid;

use crate::builder::{EventBuilder, SemanticSuffix};
use crate::codex::schema;
use crate::codex::schema::CodexRecord;

/// Regex for extracting exit codes from Codex output
/// Example: "Exit code: 0" or "Exit Code: 0" (case-insensitive)
static EXIT_CODE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)Exit Code:\s*(\d+)").unwrap());

/// Stateful Codex record -> event mapper (one per rollout file).
///
/// Handles async token notifications, JSON string parsing, and exit code extraction.
pub(crate) struct CodexRecordMapper {
    builder: EventBuilder,
    session_id: String,
    agent: AgentId,
    last_seen_model: Option<String>,
    /// Last explicit context window seen (token_count.info.model_context_window)
    last_context_window: Option<u64>,
    /// Codex often sends duplicate token_count with the same last_token_usage values
    last_seen_token_usage: Option<(u64, u64, u64)>,
    last_timestamp: Option<DateTime<Utc>>,
}

impl CodexRecordMapper {
    pub(crate) fn new(
        session_id: &str,
        agent: AgentId,
        fallback_timestamp: Option<DateTime<Utc>>,
    ) -> Self {
        // Create session_id UUID from session_id string (deterministic)
        let session_id_uuid = Uuid::new_v5(&Uuid::NAMESPACE_OID, session_id.as_bytes());
        Self {
            builder: EventBuilder::new(session_id_uuid),
            session_id: session_id.to_string(),
            agent,
            last_seen_model: None,
            last_context_window: None,
            last_seen_token_usage: None,
            last_timestamp: fallback_timestamp,
        }
    }

    /// Parse a record timestamp; unparsable timestamps inherit the last seen one.
    fn timestamp(&mut self, ts: &str) -> DateTime<Utc> {
        match DateTime::parse_from_rfc3339(ts) {
            Ok(dt) => {
                let dt = dt.with_timezone(&Utc);
                self.last_timestamp = Some(dt);
                dt
            }
            Err(_) => self.last_timestamp.unwrap_or(DateTime::<Utc>::UNIX_EPOCH),
        }
    }

    /// Map the record on `line` to events.
    pub(crate) fn map_record(
        &mut self,
        record: &CodexRecord,
        line: u64,
        byte_offset: u64,
        events: &mut Vec<AgentEvent>,
    ) {
        self.builder.begin_line(line, byte_offset);
        // Deterministic base id: session + line number (== record row for well-formed files)
        let base_id = format!("{}:row_{}", self.session_id, line);
        let agent = &self.agent.clone();

        match record {
            CodexRecord::SessionMeta(_) => {
                // Identity comes from the file header; no events.
            }

            CodexRecord::EventMsg(event_msg) => {
                let timestamp = self.timestamp(&event_msg.timestamp);

                match &event_msg.payload {
                    // user_message / agent_message / agent_reasoning are duplicated in
                    // ResponseItem with richer data.
                    schema::EventMsgPayload::UserMessage(_)
                    | schema::EventMsgPayload::AgentMessage(_)
                    | schema::EventMsgPayload::AgentReasoning(_)
                    | schema::EventMsgPayload::EnteredReviewMode(_)
                    | schema::EventMsgPayload::Unknown => {}

                    schema::EventMsgPayload::TokenCount(token_count) => {
                        // token_count only exists in event_msg, not in response_item
                        let Some(info) = &token_count.info else {
                            return;
                        };

                        if let Some(window) = info.model_context_window
                            && self.last_context_window != Some(window)
                        {
                            self.last_context_window = Some(window);
                            self.builder.build_and_push(
                                events,
                                &base_id,
                                SemanticSuffix::ContextWindowHint,
                                timestamp,
                                EventPayload::ContextWindowHint(
                                    ContextWindowHintPayload::Explicit {
                                        tokens: window,
                                        model: self.last_seen_model.clone(),
                                    },
                                ),
                                agent,
                            );
                        }

                        let usage = &info.last_token_usage;
                        let usage_triple =
                            (usage.input_tokens, usage.output_tokens, usage.total_tokens);
                        // Deduplicate repeated token_count with the same last_token_usage
                        if self.last_seen_token_usage == Some(usage_triple) {
                            return;
                        }
                        self.last_seen_token_usage = Some(usage_triple);

                        // Codex Token Conversion Rationale (codex-rs):
                        //   uncached    = input_tokens - cached_input_tokens (non_cached_input())
                        //   cache_read  = cached_input_tokens
                        //   cache_write = cache_write_input_tokens
                        //   generated   = output_tokens, reasoning = reasoning_output_tokens
                        //   tool        = 0 (Codex does not separate tool call tokens)
                        let payload = TokenUsagePayload::new(
                            TokenInput::new(
                                usage.input_tokens.saturating_sub(usage.cached_input_tokens),
                                usage.cached_input_tokens,
                                usage.cache_write_input_tokens,
                            ),
                            TokenOutput::new(usage.output_tokens, usage.reasoning_output_tokens, 0),
                        )
                        .with_model(self.last_seen_model.clone());

                        self.builder.build_and_push(
                            events,
                            &base_id,
                            SemanticSuffix::TokenUsage,
                            timestamp,
                            EventPayload::TokenUsage(payload),
                            agent,
                        );
                    }
                }
            }

            CodexRecord::ResponseItem(response_item) => {
                let timestamp = self.timestamp(&response_item.timestamp);

                match &response_item.payload {
                    schema::ResponseItemPayload::Message(message) => {
                        let text = extract_message_text(&message.content);

                        let (payload, suffix) = if message.role == "user" {
                            (
                                EventPayload::User(UserPayload { text }),
                                SemanticSuffix::User,
                            )
                        } else {
                            (
                                EventPayload::Message(MessagePayload {
                                    text,
                                    phase: message.phase.clone(),
                                }),
                                SemanticSuffix::Message,
                            )
                        };

                        self.builder
                            .build_and_push(events, &base_id, suffix, timestamp, payload, agent);
                    }

                    schema::ResponseItemPayload::Reasoning(reasoning) => {
                        let text = extract_reasoning_text(reasoning);

                        self.builder.build_and_push(
                            events,
                            &base_id,
                            SemanticSuffix::Reasoning,
                            timestamp,
                            EventPayload::Reasoning(ReasoningPayload { text }),
                            agent,
                        );
                    }

                    schema::ResponseItemPayload::FunctionCall(func_call) => {
                        let arguments = parse_json_arguments(&func_call.arguments);

                        let event_id = self.builder.build_and_push(
                            events,
                            &base_id,
                            SemanticSuffix::ToolCall,
                            timestamp,
                            EventPayload::ToolCall(super::mapper::normalize_codex_tool_call(
                                func_call.name.clone(),
                                arguments,
                                Some(func_call.call_id.clone()),
                            )),
                            agent,
                        );
                        self.builder
                            .register_tool_call(func_call.call_id.clone(), event_id);
                    }

                    schema::ResponseItemPayload::FunctionCallOutput(output) => {
                        self.push_tool_result(
                            events,
                            &base_id,
                            timestamp,
                            &output.call_id,
                            &output.output,
                        );
                    }

                    schema::ResponseItemPayload::CustomToolCall(tool_call) => {
                        let arguments = parse_json_arguments(&tool_call.input);

                        let event_id = self.builder.build_and_push(
                            events,
                            &base_id,
                            SemanticSuffix::ToolCall,
                            timestamp,
                            EventPayload::ToolCall(super::mapper::normalize_codex_tool_call(
                                tool_call.name.clone(),
                                arguments,
                                Some(tool_call.call_id.clone()),
                            )),
                            agent,
                        );
                        self.builder
                            .register_tool_call(tool_call.call_id.clone(), event_id);
                    }

                    schema::ResponseItemPayload::CustomToolCallOutput(output) => {
                        self.push_tool_result(
                            events,
                            &base_id,
                            timestamp,
                            &output.call_id,
                            &output.output,
                        );
                    }

                    schema::ResponseItemPayload::Unknown => {}
                }
            }

            CodexRecord::TurnContext(turn_context) => {
                // Track model for downstream token usage events
                if let Some(model) = &turn_context.payload.model {
                    self.last_seen_model = Some(model.clone());
                }
            }

            CodexRecord::Unknown => {}
        }
    }

    fn push_tool_result(
        &mut self,
        events: &mut Vec<AgentEvent>,
        base_id: &str,
        timestamp: DateTime<Utc>,
        call_id: &str,
        output: &str,
    ) {
        let Some(tool_call_id) = self.builder.get_tool_call_uuid(call_id) else {
            return;
        };
        let exit_code = extract_exit_code(output);
        self.builder.build_and_push(
            events,
            base_id,
            SemanticSuffix::ToolResult,
            timestamp,
            EventPayload::ToolResult(ToolResultPayload {
                output: output.to_string(),
                tool_call_id,
                is_error: exit_code.map(|code| code != 0).unwrap_or(false),
                agent_id: None,
            }),
            &self.agent,
        );
    }
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
        // Uppercase (legacy format)
        assert_eq!(extract_exit_code("Exit Code: 0"), Some(0));
        assert_eq!(extract_exit_code("Exit Code: 127"), Some(127));
        assert_eq!(extract_exit_code("Some output\nExit Code: 1\n"), Some(1));

        // Lowercase (actual Codex format)
        assert_eq!(extract_exit_code("Exit code: 0"), Some(0));
        assert_eq!(extract_exit_code("Exit code: 127"), Some(127));
        assert_eq!(extract_exit_code("Some output\nExit code: 1\n"), Some(1));

        // Mixed case
        assert_eq!(extract_exit_code("EXIT CODE: 42"), Some(42));

        // No match
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
