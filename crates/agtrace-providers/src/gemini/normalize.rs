use agtrace_types::*;
use chrono::DateTime;
use uuid::Uuid;

use crate::builder::{EventBuilder, SemanticSuffix};
use crate::gemini::schema::{GeminiMessage, GeminiSession};
use crate::gemini::tools::{
    GeminiGoogleWebSearchArgs, GeminiReadFileArgs, GeminiReplaceArgs, GeminiRunShellCommandArgs,
    GeminiWriteFileArgs, GeminiWriteTodosArgs,
};

/// Normalize Gemini-specific tool calls
///
/// Handles Gemini provider-specific tool names and maps them to domain variants.
/// Uses provider-specific Args structs for proper schema parsing and conversion.
pub(crate) fn normalize_gemini_tool_call(
    tool_name: String,
    arguments: serde_json::Value,
    provider_call_id: Option<String>,
) -> ToolCallPayload {
    // Handle Gemini-specific tools with provider-specific types
    match tool_name.as_str() {
        "read_file" => {
            // Parse as Gemini-specific Args, then convert to domain model
            if let Ok(gemini_args) = serde_json::from_value::<GeminiReadFileArgs>(arguments.clone())
            {
                return ToolCallPayload::FileRead {
                    name: tool_name,
                    arguments: gemini_args.to_file_read_args(),
                    provider_call_id,
                };
            }
        }
        "write_file" => {
            // Parse as Gemini-specific Args, then convert to domain model
            if let Ok(gemini_args) =
                serde_json::from_value::<GeminiWriteFileArgs>(arguments.clone())
            {
                return ToolCallPayload::FileWrite {
                    name: tool_name,
                    arguments: gemini_args.to_file_write_args(),
                    provider_call_id,
                };
            }
        }
        "replace" => {
            // Parse as Gemini-specific Args (with instruction field), then convert
            if let Ok(gemini_args) = serde_json::from_value::<GeminiReplaceArgs>(arguments.clone())
            {
                return ToolCallPayload::FileEdit {
                    name: tool_name,
                    arguments: gemini_args.to_file_edit_args(),
                    provider_call_id,
                };
            }
        }
        "run_shell_command" => {
            // Parse as Gemini-specific Args, then convert to domain model
            if let Ok(gemini_args) =
                serde_json::from_value::<GeminiRunShellCommandArgs>(arguments.clone())
            {
                return ToolCallPayload::Execute {
                    name: tool_name,
                    arguments: gemini_args.to_execute_args(),
                    provider_call_id,
                };
            }
        }
        "google_web_search" => {
            // Parse as Gemini-specific Args, then convert to domain model
            if let Ok(gemini_args) =
                serde_json::from_value::<GeminiGoogleWebSearchArgs>(arguments.clone())
            {
                return ToolCallPayload::Search {
                    name: tool_name,
                    arguments: gemini_args.to_search_args(),
                    provider_call_id,
                };
            }
        }
        "write_todos" => {
            // Validate as Gemini-specific Args, then keep as Generic
            // (no unified Plan variant exists yet in domain model)
            if serde_json::from_value::<GeminiWriteTodosArgs>(arguments.clone()).is_ok() {
                return ToolCallPayload::Generic {
                    name: tool_name,
                    arguments,
                    provider_call_id,
                };
            }
        }
        _ if tool_name.starts_with("mcp__") => {
            // MCP tools
            if let Ok(args) = serde_json::from_value(arguments.clone()) {
                return ToolCallPayload::Mcp {
                    name: tool_name,
                    arguments: args,
                    provider_call_id,
                };
            }
        }
        _ => {
            // Unknown Gemini tool, fall through to Generic
        }
    }

    // Fallback to generic
    ToolCallPayload::Generic {
        name: tool_name,
        arguments,
        provider_call_id,
    }
}

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
                        EventPayload::ToolCall(normalize_gemini_tool_call(
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

        let events = normalize_gemini_session(&session, vec![]);
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

    #[test]
    fn test_normalize_read_file() {
        let payload = normalize_gemini_tool_call(
            "read_file".to_string(),
            serde_json::json!({"file_path": "src/main.rs"}),
            Some("call_123".to_string()),
        );

        match payload {
            ToolCallPayload::FileRead {
                name,
                arguments,
                provider_call_id,
            } => {
                assert_eq!(name, "read_file");
                assert_eq!(arguments.file_path, Some("src/main.rs".to_string()));
                assert_eq!(provider_call_id, Some("call_123".to_string()));
            }
            _ => panic!("Expected FileRead variant"),
        }
    }

    #[test]
    fn test_normalize_write_file() {
        let payload = normalize_gemini_tool_call(
            "write_file".to_string(),
            serde_json::json!({"file_path": "test.txt", "content": "hello"}),
            Some("call_456".to_string()),
        );

        match payload {
            ToolCallPayload::FileWrite {
                name,
                arguments,
                provider_call_id,
            } => {
                assert_eq!(name, "write_file");
                assert_eq!(arguments.file_path, "test.txt");
                assert_eq!(arguments.content, "hello");
                assert_eq!(provider_call_id, Some("call_456".to_string()));
            }
            _ => panic!("Expected FileWrite variant"),
        }
    }

    #[test]
    fn test_normalize_replace() {
        let payload = normalize_gemini_tool_call(
            "replace".to_string(),
            serde_json::json!({
                "file_path": "src/lib.rs",
                "old_string": "old",
                "new_string": "new",
                "replace_all": false
            }),
            Some("call_789".to_string()),
        );

        match payload {
            ToolCallPayload::FileEdit {
                name,
                arguments,
                provider_call_id,
            } => {
                assert_eq!(name, "replace");
                assert_eq!(arguments.file_path, "src/lib.rs");
                assert_eq!(arguments.old_string, "old");
                assert_eq!(arguments.new_string, "new");
                assert_eq!(provider_call_id, Some("call_789".to_string()));
            }
            _ => panic!("Expected FileEdit variant"),
        }
    }

    #[test]
    fn test_normalize_run_shell_command() {
        let payload = normalize_gemini_tool_call(
            "run_shell_command".to_string(),
            serde_json::json!({"command": "ls", "description": "list files"}),
            Some("call_abc".to_string()),
        );

        match payload {
            ToolCallPayload::Execute {
                name,
                arguments,
                provider_call_id,
            } => {
                assert_eq!(name, "run_shell_command");
                assert_eq!(arguments.command, Some("ls".to_string()));
                assert_eq!(arguments.description, Some("list files".to_string()));
                assert_eq!(provider_call_id, Some("call_abc".to_string()));
            }
            _ => panic!("Expected Execute variant"),
        }
    }

    #[test]
    fn test_normalize_write_todos() {
        let payload = normalize_gemini_tool_call(
            "write_todos".to_string(),
            serde_json::json!({
                "todos": [
                    {
                        "description": "Create cli directory",
                        "status": "pending"
                    },
                    {
                        "description": "Move import logic",
                        "status": "in_progress"
                    }
                ]
            }),
            Some("call_todos".to_string()),
        );

        match payload {
            ToolCallPayload::Generic {
                name,
                arguments,
                provider_call_id,
            } => {
                assert_eq!(name, "write_todos");
                assert_eq!(provider_call_id, Some("call_todos".to_string()));
                // Verify the arguments structure is preserved
                let todos = arguments.get("todos").unwrap().as_array().unwrap();
                assert_eq!(todos.len(), 2);
                assert_eq!(
                    todos[0].get("description").unwrap().as_str().unwrap(),
                    "Create cli directory"
                );
                assert_eq!(todos[0].get("status").unwrap().as_str().unwrap(), "pending");
            }
            _ => panic!("Expected Generic variant for write_todos (no Plan variant yet)"),
        }
    }

    #[test]
    fn test_normalize_unknown_gemini_tool() {
        let payload = normalize_gemini_tool_call(
            "unknown_tool".to_string(),
            serde_json::json!({"foo": "bar"}),
            Some("call_999".to_string()),
        );

        match payload {
            ToolCallPayload::Generic {
                name,
                arguments,
                provider_call_id,
            } => {
                assert_eq!(name, "unknown_tool");
                assert_eq!(arguments, serde_json::json!({"foo": "bar"}));
                assert_eq!(provider_call_id, Some("call_999".to_string()));
            }
            _ => panic!("Expected Generic variant for unknown tool"),
        }
    }
}
