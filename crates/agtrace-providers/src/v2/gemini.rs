use agtrace_types::v2::*;
use chrono::DateTime;
use uuid::Uuid;

use super::builder::EventBuilder;
use crate::gemini::schema::{GeminiMessage, GeminiSession};

/// Normalize Gemini session to v2 events
/// Unfolds nested structure (thoughts[], toolCalls[]) into event stream
pub fn normalize_gemini_session_v2(
    session: &GeminiSession,
    raw_messages: Vec<serde_json::Value>,
) -> Vec<AgentEvent> {
    // Create trace_id from session_id (deterministic)
    let trace_id = Uuid::new_v5(&Uuid::NAMESPACE_OID, session.session_id.as_bytes());
    let mut builder = EventBuilder::new(trace_id);
    let mut events = Vec::new();

    for (idx, msg) in session.messages.iter().enumerate() {
        let raw_value = raw_messages
            .get(idx)
            .cloned()
            .unwrap_or(serde_json::Value::Null);

        match msg {
            GeminiMessage::User(user_msg) => {
                // Skip numeric IDs (legacy CLI events)
                if user_msg.id.parse::<u32>().is_ok() {
                    continue;
                }

                let timestamp = parse_timestamp(&user_msg.timestamp);
                let event = builder.create_event(
                    timestamp,
                    EventPayload::User(UserPayload {
                        text: user_msg.content.clone(),
                    }),
                    Some(raw_value),
                );
                events.push(event);
            }

            GeminiMessage::Gemini(gemini_msg) => {
                let timestamp = parse_timestamp(&gemini_msg.timestamp);

                // 1. Reasoning events (thoughts)
                for thought in &gemini_msg.thoughts {
                    let event = builder.create_event(
                        timestamp,
                        EventPayload::Reasoning(ReasoningPayload {
                            text: format!("{}: {}", thought.subject, thought.description),
                        }),
                        Some(raw_value.clone()),
                    );
                    events.push(event);
                }

                // 2. Tool calls and results
                for tool_call in &gemini_msg.tool_calls {
                    // ToolCall event
                    let tool_event = builder.create_event(
                        timestamp,
                        EventPayload::ToolCall(ToolCallPayload {
                            name: tool_call.name.clone(),
                            arguments: tool_call.args.clone(),
                        }),
                        Some(raw_value.clone()),
                    );

                    // Save tool_call UUID before moving tool_event
                    let tool_call_uuid = tool_event.id;

                    // Register tool call ID mapping (provider ID -> UUID)
                    builder.register_tool_call(tool_call.id.clone(), tool_call_uuid);
                    events.push(tool_event);

                    // ToolResult event (if result exists)
                    if !tool_call.result.is_empty() {
                        let output = tool_call
                            .result_display
                            .clone()
                            .unwrap_or_else(|| format!("{:?}", tool_call.result));

                        let is_error = tool_call
                            .status
                            .as_ref()
                            .map(|s| s == "error")
                            .unwrap_or(false);

                        let result_event = builder.create_event(
                            timestamp,
                            EventPayload::ToolResult(ToolResultPayload {
                                output,
                                tool_call_id: tool_call_uuid, // Reference to ToolCall UUID
                                is_error,
                            }),
                            Some(raw_value.clone()),
                        );
                        events.push(result_event);
                    }
                }

                // 3. Message event (assistant response)
                let message_event = builder.create_event(
                    timestamp,
                    EventPayload::Message(MessagePayload {
                        text: gemini_msg.content.clone(),
                    }),
                    Some(raw_value.clone()),
                );
                events.push(message_event.clone());

                // 4. TokenUsage event (sidecar attached to message)
                // Gemini returns turn-level totals, so we attach to the last generation event
                let token_event = builder.create_event(
                    timestamp,
                    EventPayload::TokenUsage(TokenUsagePayload {
                        input_tokens: gemini_msg.tokens.input as i32,
                        output_tokens: gemini_msg.tokens.output as i32,
                        total_tokens: gemini_msg.tokens.total as i32,
                        details: Some(TokenUsageDetails {
                            cache_read_input_tokens: Some(gemini_msg.tokens.cached as i32),
                            reasoning_output_tokens: Some(gemini_msg.tokens.thoughts as i32),
                        }),
                    }),
                    None, // No raw for token event
                );
                events.push(token_event);
            }

            GeminiMessage::Info(_info_msg) => {
                // Skip info messages for now
                // Could map to system events in future if needed
            }
        }
    }

    events
}

/// Parse Gemini timestamp to DateTime<Utc>
fn parse_timestamp(ts: &str) -> DateTime<chrono::Utc> {
    DateTime::parse_from_rfc3339(ts)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gemini::schema::{GeminiAssistantMessage, TokenUsage, UserMessage};

    #[test]
    fn test_normalize_user_message() {
        let session = GeminiSession {
            session_id: "test-session".to_string(),
            project_hash: "test-hash".to_string(),
            start_time: "2024-01-01T00:00:00Z".to_string(),
            last_updated: "2024-01-01T00:00:00Z".to_string(),
            messages: vec![GeminiMessage::User(UserMessage {
                id: "uuid-123".to_string(),
                timestamp: "2024-01-01T00:00:00Z".to_string(),
                content: "Hello".to_string(),
            })],
        };

        let events = normalize_gemini_session_v2(&session, vec![]);
        assert_eq!(events.len(), 1);

        match &events[0].payload {
            EventPayload::User(payload) => assert_eq!(payload.text, "Hello"),
            _ => panic!("Expected User payload"),
        }
        assert_eq!(events[0].parent_id, None);
    }

    #[test]
    fn test_normalize_assistant_with_tokens() {
        let session = GeminiSession {
            session_id: "test-session".to_string(),
            project_hash: "test-hash".to_string(),
            start_time: "2024-01-01T00:00:00Z".to_string(),
            last_updated: "2024-01-01T00:00:00Z".to_string(),
            messages: vec![GeminiMessage::Gemini(GeminiAssistantMessage {
                id: "msg-1".to_string(),
                timestamp: "2024-01-01T00:00:00Z".to_string(),
                content: "Response".to_string(),
                model: "gemini-pro".to_string(),
                thoughts: vec![],
                tool_calls: vec![],
                tokens: TokenUsage {
                    input: 100,
                    output: 50,
                    total: 150,
                    cached: 10,
                    thoughts: 5,
                    tool: 0,
                },
            })],
        };

        let events = normalize_gemini_session_v2(&session, vec![]);
        // Should have: Message + TokenUsage (2 events)
        assert_eq!(events.len(), 2);

        match &events[0].payload {
            EventPayload::Message(payload) => assert_eq!(payload.text, "Response"),
            _ => panic!("Expected Message payload"),
        }

        match &events[1].payload {
            EventPayload::TokenUsage(payload) => {
                assert_eq!(payload.input_tokens, 100);
                assert_eq!(payload.output_tokens, 50);
                assert_eq!(payload.total_tokens, 150);
            }
            _ => panic!("Expected TokenUsage payload"),
        }

        // TokenUsage parent is Message
        assert_eq!(events[1].parent_id, Some(events[0].id));
    }
}
