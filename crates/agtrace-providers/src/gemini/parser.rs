use agtrace_types::*;
use anyhow::Result;
use chrono::DateTime;
use std::path::Path;
use uuid::Uuid;

use crate::builder::{EventBuilder, SemanticSuffix};
use crate::gemini::schema::{GeminiMessage, GeminiSession};

/// Normalize Gemini session to events
/// Unfolds nested structure (thoughts[], toolCalls[]) into event stream
pub(crate) fn normalize_gemini_session(
    session: &GeminiSession,
    raw_messages: Vec<serde_json::Value>,
) -> Vec<AgentEvent> {
    // Create session_id UUID from session_id string (deterministic)
    let session_id_uuid = Uuid::new_v5(&Uuid::NAMESPACE_OID, session.session_id.as_bytes());
    let mut builder = EventBuilder::new(session_id_uuid);
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
                builder.build_and_push(
                    &mut events,
                    &user_msg.id,
                    SemanticSuffix::User,
                    timestamp,
                    EventPayload::User(UserPayload {
                        text: user_msg.content.clone(),
                    }),
                    Some(raw_value),
                    StreamId::Main,
                );
            }

            GeminiMessage::Gemini(gemini_msg) => {
                let timestamp = parse_timestamp(&gemini_msg.timestamp);
                let base_id = &gemini_msg.id;

                // 1. Reasoning events (thoughts)
                for (idx, thought) in gemini_msg.thoughts.iter().enumerate() {
                    let indexed_base_id = format!("{}-thought-{}", base_id, idx);
                    builder.build_and_push(
                        &mut events,
                        &indexed_base_id,
                        SemanticSuffix::Reasoning,
                        timestamp,
                        EventPayload::Reasoning(ReasoningPayload {
                            text: format!("{}: {}", thought.subject, thought.description),
                        }),
                        Some(raw_value.clone()),
                        StreamId::Main,
                    );
                }

                // 2. Tool calls and results
                for (idx, tool_call) in gemini_msg.tool_calls.iter().enumerate() {
                    let indexed_base_id = format!("{}-tool-{}", base_id, idx);

                    // ToolCall event
                    let tool_call_uuid = builder.build_and_push(
                        &mut events,
                        &indexed_base_id,
                        SemanticSuffix::ToolCall,
                        timestamp,
                        EventPayload::ToolCall(super::mapper::normalize_gemini_tool_call(
                            tool_call.name.clone(),
                            tool_call.args.clone(),
                            Some(tool_call.id.clone()),
                        )),
                        Some(raw_value.clone()),
                        StreamId::Main,
                    );

                    // Register tool call ID mapping (provider ID -> UUID)
                    builder.register_tool_call(tool_call.id.clone(), tool_call_uuid);

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

                        builder.build_and_push(
                            &mut events,
                            &indexed_base_id,
                            SemanticSuffix::ToolResult,
                            timestamp,
                            EventPayload::ToolResult(ToolResultPayload {
                                output,
                                tool_call_id: tool_call_uuid, // Reference to ToolCall UUID
                                is_error,
                            }),
                            Some(raw_value.clone()),
                            StreamId::Main,
                        );
                    }
                }

                // 3. Message event (assistant response)
                builder.build_and_push(
                    &mut events,
                    base_id,
                    SemanticSuffix::Message,
                    timestamp,
                    EventPayload::Message(MessagePayload {
                        text: gemini_msg.content.clone(),
                    }),
                    Some(raw_value.clone()),
                    StreamId::Main,
                );

                // 4. TokenUsage event (sidecar attached to message)
                // Gemini returns turn-level totals, so we attach to the last generation event
                builder.build_and_push(
                    &mut events,
                    base_id,
                    SemanticSuffix::TokenUsage,
                    timestamp,
                    EventPayload::TokenUsage(TokenUsagePayload {
                        input_tokens: gemini_msg.tokens.input as i32,
                        output_tokens: gemini_msg.tokens.output as i32,
                        total_tokens: gemini_msg.tokens.total as i32,
                        details: Some(TokenUsageDetails {
                            cache_creation_input_tokens: None, // Gemini doesn't track cache creation separately
                            cache_read_input_tokens: Some(gemini_msg.tokens.cached as i32),
                            reasoning_output_tokens: Some(gemini_msg.tokens.thoughts as i32),
                        }),
                    }),
                    Some(raw_value),
                    StreamId::Main,
                );
            }

            GeminiMessage::Info(info_msg) => {
                let timestamp = parse_timestamp(&info_msg.timestamp);
                builder.build_and_push(
                    &mut events,
                    &info_msg.id,
                    SemanticSuffix::Notification,
                    timestamp,
                    EventPayload::Notification(NotificationPayload {
                        text: info_msg.content.clone(),
                        level: Some("info".to_string()),
                    }),
                    Some(raw_value),
                    StreamId::Main,
                );
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

/// Gemini session parser implementation
pub struct GeminiParser;

impl crate::traits::SessionParser for GeminiParser {
    fn parse_file(&self, path: &Path) -> Result<Vec<AgentEvent>> {
        super::io::normalize_gemini_file(path)
    }

    fn parse_record(&self, content: &str) -> Result<Option<AgentEvent>> {
        // Gemini uses JSON format (not JSONL), parse as AgentEvent
        match serde_json::from_str::<AgentEvent>(content) {
            Ok(event) => Ok(Some(event)),
            Err(_) => Ok(None), // Skip malformed lines
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gemini::schema::{GeminiAssistantMessage, TokenUsage, UserMessage};

    #[test]
    fn test_normalize_user_message() {
        let session = GeminiSession {
            session_id: "test-session".to_string(),
            project_hash: agtrace_types::ProjectHash::from("test-hash"),
            start_time: "2024-01-01T00:00:00Z".to_string(),
            last_updated: "2024-01-01T00:00:00Z".to_string(),
            messages: vec![GeminiMessage::User(UserMessage {
                id: "uuid-123".to_string(),
                timestamp: "2024-01-01T00:00:00Z".to_string(),
                content: "Hello".to_string(),
            })],
        };

        let events = normalize_gemini_session(&session, vec![]);
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
            project_hash: agtrace_types::ProjectHash::from("test-hash"),
            start_time: "2024-01-01T00:00:00Z".to_string(),
            last_updated: "2024-01-01T00:00:00Z".to_string(),
            messages: vec![GeminiMessage::Gemini(GeminiAssistantMessage {
                id: "uuid-456".to_string(),
                timestamp: "2024-01-01T00:00:01Z".to_string(),
                content: "Hello back!".to_string(),
                model: "gemini-2.0-flash".to_string(),
                thoughts: vec![],
                tool_calls: vec![],
                tokens: TokenUsage {
                    input: 10,
                    output: 5,
                    total: 15,
                    cached: 2,
                    thoughts: 1,
                    tool: 0,
                },
            })],
        };

        let events = normalize_gemini_session(&session, vec![]);
        // Should have: Message + TokenUsage (2 events)
        assert_eq!(events.len(), 2);

        match &events[0].payload {
            EventPayload::Message(payload) => assert_eq!(payload.text, "Hello back!"),
            _ => panic!("Expected Message payload"),
        }

        match &events[1].payload {
            EventPayload::TokenUsage(payload) => {
                assert_eq!(payload.input_tokens, 10);
                assert_eq!(payload.output_tokens, 5);
                assert_eq!(payload.total_tokens, 15);
                assert_eq!(
                    payload.details.as_ref().unwrap().cache_read_input_tokens,
                    Some(2)
                );
                assert_eq!(
                    payload.details.as_ref().unwrap().reasoning_output_tokens,
                    Some(1)
                );
            }
            _ => panic!("Expected TokenUsage payload"),
        }
    }
}
