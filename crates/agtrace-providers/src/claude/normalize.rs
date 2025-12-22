use agtrace_types::*;
use chrono::DateTime;
use uuid::Uuid;

use crate::builder::{EventBuilder, SemanticSuffix};
use crate::claude::schema::*;
use crate::normalization::normalize_tool_call;

/// Determine StreamId from Claude record fields
fn determine_stream_id(is_sidechain: bool, agent_id: &Option<String>) -> StreamId {
    if is_sidechain {
        StreamId::Sidechain {
            agent_id: agent_id.clone().unwrap_or_else(|| "unknown".to_string()),
        }
    } else {
        StreamId::Main
    }
}

/// Normalize Claude session records to events
/// Handles message.content[] blocks, thinking -> Reasoning, and TokenUsage extraction
pub(crate) fn normalize_claude_session(records: Vec<ClaudeRecord>) -> Vec<AgentEvent> {
    // Extract session_id from first record
    let session_id = records
        .iter()
        .find_map(|r| match r {
            ClaudeRecord::User(user) => Some(user.session_id.clone()),
            ClaudeRecord::Assistant(asst) => Some(asst.session_id.clone()),
            _ => None,
        })
        .unwrap_or_else(|| "unknown".to_string());

    // Create session_id UUID from session_id string (deterministic)
    let session_id_uuid = Uuid::new_v5(&Uuid::NAMESPACE_OID, session_id.as_bytes());
    let mut builder = EventBuilder::new(session_id_uuid);
    let mut events = Vec::new();

    for record in records {
        match record {
            ClaudeRecord::User(user_record) => {
                let timestamp = parse_timestamp(&user_record.timestamp);
                let raw_value = serde_json::to_value(&user_record).ok();
                let base_id = &user_record.uuid;
                let stream_id =
                    determine_stream_id(user_record.is_sidechain, &user_record.agent_id);

                // Process user content blocks
                for (idx, content) in user_record.message.content.iter().enumerate() {
                    let indexed_base_id = format!("{}-content-{}", base_id, idx);

                    match content {
                        UserContent::Text { text } => {
                            builder.build_and_push(
                                &mut events,
                                &indexed_base_id,
                                SemanticSuffix::User,
                                timestamp,
                                EventPayload::User(UserPayload { text: text.clone() }),
                                raw_value.clone(),
                                stream_id.clone(),
                            );
                        }

                        UserContent::ToolResult {
                            tool_use_id,
                            content: result_content,
                            is_error,
                        } => {
                            // ToolResult in user message - map to ToolResult event
                            // Need to look up the tool_call_id from provider ID
                            if let Some(tool_call_id) = builder.get_tool_call_uuid(tool_use_id) {
                                let output = result_content
                                    .as_ref()
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();

                                builder.build_and_push(
                                    &mut events,
                                    &indexed_base_id,
                                    SemanticSuffix::ToolResult,
                                    timestamp,
                                    EventPayload::ToolResult(ToolResultPayload {
                                        output,
                                        tool_call_id,
                                        is_error: *is_error,
                                    }),
                                    raw_value.clone(),
                                    stream_id.clone(),
                                );
                            }
                        }

                        UserContent::Image { .. } => {
                            // Skip image content for now
                            // Could map to metadata in future
                        }

                        UserContent::Unknown => {
                            // Skip unknown content types
                        }
                    }
                }
            }

            ClaudeRecord::Assistant(asst_record) => {
                let timestamp = parse_timestamp(&asst_record.timestamp);
                let raw_value = serde_json::to_value(&asst_record).ok();
                let base_id = &asst_record.uuid;
                let stream_id =
                    determine_stream_id(asst_record.is_sidechain, &asst_record.agent_id);

                // Track the last generation event for TokenUsage sidecar
                let mut last_generation_event_id: Option<Uuid> = None;

                // Process assistant content blocks
                for (idx, content) in asst_record.message.content.iter().enumerate() {
                    let indexed_base_id = format!("{}-content-{}", base_id, idx);

                    match content {
                        AssistantContent::Thinking { thinking, .. } => {
                            // Thinking block -> Reasoning event
                            builder.build_and_push(
                                &mut events,
                                &indexed_base_id,
                                SemanticSuffix::Reasoning,
                                timestamp,
                                EventPayload::Reasoning(ReasoningPayload {
                                    text: thinking.clone(),
                                }),
                                raw_value.clone(),
                                stream_id.clone(),
                            );
                        }

                        AssistantContent::ToolUse {
                            id, name, input, ..
                        } => {
                            // ToolUse -> ToolCall event
                            let event_id = builder.build_and_push(
                                &mut events,
                                &indexed_base_id,
                                SemanticSuffix::ToolCall,
                                timestamp,
                                EventPayload::ToolCall(normalize_tool_call(
                                    name.clone(),
                                    input.clone(),
                                    Some(id.clone()),
                                )),
                                raw_value.clone(),
                                stream_id.clone(),
                            );

                            // Register tool call mapping
                            builder.register_tool_call(id.clone(), event_id);
                            last_generation_event_id = Some(event_id);
                        }

                        AssistantContent::Text { text, .. } => {
                            // Text block -> Message event
                            let event_id = builder.build_and_push(
                                &mut events,
                                &indexed_base_id,
                                SemanticSuffix::Message,
                                timestamp,
                                EventPayload::Message(MessagePayload { text: text.clone() }),
                                raw_value.clone(),
                                stream_id.clone(),
                            );
                            last_generation_event_id = Some(event_id);
                        }

                        AssistantContent::ToolResult {
                            tool_use_id,
                            content: output,
                            is_error,
                        } => {
                            // ToolResult in assistant content (rare, but handle it)
                            if let Some(tool_call_id) = builder.get_tool_call_uuid(tool_use_id) {
                                builder.build_and_push(
                                    &mut events,
                                    &indexed_base_id,
                                    SemanticSuffix::ToolResult,
                                    timestamp,
                                    EventPayload::ToolResult(ToolResultPayload {
                                        output: output.clone(),
                                        tool_call_id,
                                        is_error: *is_error,
                                    }),
                                    raw_value.clone(),
                                    stream_id.clone(),
                                );
                            }
                        }

                        AssistantContent::Unknown => {
                            // Skip unknown content types
                        }
                    }
                }

                // Extract TokenUsage from message.usage
                if let Some(usage) = &asst_record.message.usage {
                    // Attach to last generation event (ToolCall or Message)
                    if last_generation_event_id.is_some() {
                        builder.build_and_push(
                            &mut events,
                            base_id,
                            SemanticSuffix::TokenUsage,
                            timestamp,
                            EventPayload::TokenUsage(TokenUsagePayload {
                                input_tokens: usage.input_tokens as i32,
                                output_tokens: usage.output_tokens as i32,
                                total_tokens: (usage.input_tokens + usage.output_tokens) as i32,
                                details: Some(TokenUsageDetails {
                                    cache_creation_input_tokens: usage
                                        .cache_creation_input_tokens
                                        .map(|t| t as i32),
                                    cache_read_input_tokens: usage
                                        .cache_read_input_tokens
                                        .map(|t| t as i32),
                                    reasoning_output_tokens: None, // Claude doesn't separate reasoning tokens
                                }),
                            }),
                            raw_value.clone(),
                            stream_id.clone(),
                        );
                    }
                }
            }

            ClaudeRecord::FileHistorySnapshot(_snapshot) => {
                // Skip file snapshots for now (file system events)
            }

            ClaudeRecord::Unknown => {
                // Skip unknown record types
            }
        }
    }

    events
}

/// Parse Claude timestamp to DateTime<Utc>
fn parse_timestamp(ts: &str) -> DateTime<chrono::Utc> {
    DateTime::parse_from_rfc3339(ts)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_user_message() {
        let records = vec![ClaudeRecord::User(UserRecord {
            uuid: "uuid-123".to_string(),
            parent_uuid: None,
            session_id: "session-1".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            message: UserMessage {
                role: "user".to_string(),
                content: vec![UserContent::Text {
                    text: "Hello".to_string(),
                }],
            },
            is_sidechain: false,
            is_meta: false,
            agent_id: None,
            cwd: None,
            git_branch: None,
            user_type: None,
            version: None,
            thinking_metadata: None,
        })];

        let events = normalize_claude_session(records);
        assert_eq!(events.len(), 1);

        match &events[0].payload {
            EventPayload::User(payload) => assert_eq!(payload.text, "Hello"),
            _ => panic!("Expected User payload"),
        }
    }

    #[test]
    fn test_normalize_assistant_with_thinking() {
        let records = vec![ClaudeRecord::Assistant(AssistantRecord {
            uuid: "uuid-123".to_string(),
            parent_uuid: None,
            session_id: "session-1".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            message: AssistantMessage {
                message_type: "message".to_string(),
                id: "msg-1".to_string(),
                role: "assistant".to_string(),
                model: "claude-3-5-sonnet-20241022".to_string(),
                content: vec![
                    AssistantContent::Thinking {
                        thinking: "Let me think...".to_string(),
                        signature: None,
                    },
                    AssistantContent::Text {
                        text: "Here's the answer".to_string(),
                        signature: None,
                    },
                ],
                stop_reason: Some("end_turn".to_string()),
                stop_sequence: None,
                usage: Some(TokenUsage {
                    input_tokens: 100,
                    output_tokens: 50,
                    cache_creation_input_tokens: None,
                    cache_read_input_tokens: Some(10),
                }),
            },
            is_sidechain: false,
            agent_id: None,
            cwd: None,
            git_branch: None,
            user_type: None,
            version: None,
            request_id: None,
        })];

        let events = normalize_claude_session(records);
        // Should have: Reasoning + Message + TokenUsage (3 events)
        assert_eq!(events.len(), 3);

        match &events[0].payload {
            EventPayload::Reasoning(payload) => assert_eq!(payload.text, "Let me think..."),
            _ => panic!("Expected Reasoning payload"),
        }

        match &events[1].payload {
            EventPayload::Message(payload) => assert_eq!(payload.text, "Here's the answer"),
            _ => panic!("Expected Message payload"),
        }

        match &events[2].payload {
            EventPayload::TokenUsage(payload) => {
                assert_eq!(payload.input_tokens, 100);
                assert_eq!(payload.output_tokens, 50);
                assert_eq!(
                    payload
                        .details
                        .as_ref()
                        .unwrap()
                        .cache_creation_input_tokens,
                    None
                );
                assert_eq!(
                    payload.details.as_ref().unwrap().cache_read_input_tokens,
                    Some(10)
                );
            }
            _ => panic!("Expected TokenUsage payload"),
        }
    }

    #[test]
    fn test_normalize_tool_use() {
        let records = vec![ClaudeRecord::Assistant(AssistantRecord {
            uuid: "uuid-123".to_string(),
            parent_uuid: None,
            session_id: "session-1".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            message: AssistantMessage {
                message_type: "message".to_string(),
                id: "msg-1".to_string(),
                role: "assistant".to_string(),
                model: "claude-3-5-sonnet-20241022".to_string(),
                content: vec![AssistantContent::ToolUse {
                    id: "toolu_123".to_string(),
                    name: "bash".to_string(),
                    input: serde_json::json!({"command": "ls"}),
                    signature: None,
                }],
                stop_reason: Some("tool_use".to_string()),
                stop_sequence: None,
                usage: Some(TokenUsage {
                    input_tokens: 50,
                    output_tokens: 20,
                    cache_creation_input_tokens: None,
                    cache_read_input_tokens: None,
                }),
            },
            is_sidechain: false,
            agent_id: None,
            cwd: None,
            git_branch: None,
            user_type: None,
            version: None,
            request_id: None,
        })];

        let events = normalize_claude_session(records);
        // Should have: ToolCall + TokenUsage (2 events)
        assert_eq!(events.len(), 2);

        match &events[0].payload {
            EventPayload::ToolCall(payload) => {
                assert_eq!(payload.name(), "bash");
                assert_eq!(payload.provider_call_id(), Some("toolu_123"));
            }
            _ => panic!("Expected ToolCall payload"),
        }

        match &events[1].payload {
            EventPayload::TokenUsage(_) => {}
            _ => panic!("Expected TokenUsage payload"),
        }
    }
}
