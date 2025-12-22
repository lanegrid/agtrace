use agtrace_types::{self, *};
use chrono::DateTime;
use regex::Regex;
use std::sync::LazyLock;
use uuid::Uuid;

use crate::builder::{EventBuilder, SemanticSuffix};
use crate::codex::schema;
use crate::codex::schema::CodexRecord;
use crate::codex::tools::{ApplyPatchArgs, PatchOperation, ReadMcpResourceArgs, ShellArgs};

/// Regex for extracting exit codes from Codex output
/// Example: "Exit Code: 0" or similar patterns
static EXIT_CODE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"Exit Code:\s*(\d+)").unwrap());

/// Attach model to event metadata when available
fn attach_model_metadata(
    metadata: Option<serde_json::Value>,
    model: Option<&String>,
) -> Option<serde_json::Value> {
    let model = match model {
        Some(m) => m.clone(),
        None => return metadata,
    };

    match metadata {
        Some(serde_json::Value::Object(mut map)) => {
            map.entry("model")
                .or_insert_with(|| serde_json::Value::String(model));
            Some(serde_json::Value::Object(map))
        }
        Some(other) => {
            let mut map = serde_json::Map::new();
            map.insert("raw".to_string(), other);
            map.insert("model".to_string(), serde_json::Value::String(model));
            Some(serde_json::Value::Object(map))
        }
        None => {
            let mut map = serde_json::Map::new();
            map.insert("model".to_string(), serde_json::Value::String(model));
            Some(serde_json::Value::Object(map))
        }
    }
}

/// Normalize Codex-specific tool calls
///
/// Handles provider-specific tools like apply_patch before falling back to generic normalization.
fn normalize_codex_tool_call(
    tool_name: String,
    arguments: serde_json::Value,
    provider_call_id: Option<String>,
) -> ToolCallPayload {
    // Handle Codex-specific tools
    match tool_name.as_str() {
        "apply_patch" => {
            // Try to parse as ApplyPatchArgs
            if let Ok(patch_args) = serde_json::from_value::<ApplyPatchArgs>(arguments.clone()) {
                // Parse the patch structure
                match patch_args.parse() {
                    Ok(parsed) => {
                        // Map to FileWrite (Add) or FileEdit (Update) based on operation
                        match parsed.operation {
                            PatchOperation::Add => {
                                // New file creation → FileWrite
                                return ToolCallPayload::FileWrite {
                                    name: tool_name,
                                    arguments: FileWriteArgs {
                                        file_path: parsed.file_path,
                                        content: parsed.raw_patch,
                                    },
                                    provider_call_id,
                                };
                            }
                            PatchOperation::Update => {
                                // File modification → FileEdit
                                // Note: For patches, we store the raw patch in old_string/new_string
                                // as a placeholder. The actual diff is in the raw patch.
                                return ToolCallPayload::FileEdit {
                                    name: tool_name,
                                    arguments: FileEditArgs {
                                        file_path: parsed.file_path,
                                        old_string: String::new(), // Placeholder: actual diff in raw patch
                                        new_string: parsed.raw_patch.clone(),
                                        replace_all: false,
                                    },
                                    provider_call_id,
                                };
                            }
                        }
                    }
                    Err(_) => {
                        // Parsing failed, fall back to generic
                    }
                }
            }
        }
        "shell" => {
            // Try to parse as ShellArgs
            if let Ok(shell_args) = serde_json::from_value::<ShellArgs>(arguments.clone()) {
                // Convert to standard ExecuteArgs
                let execute_args = shell_args.to_execute_args();
                return ToolCallPayload::Execute {
                    name: tool_name,
                    arguments: execute_args,
                    provider_call_id,
                };
            }
        }
        "read_mcp_resource" => {
            // Try to parse as ReadMcpResourceArgs
            if let Ok(mcp_args) = serde_json::from_value::<ReadMcpResourceArgs>(arguments.clone()) {
                // Convert to standard FileReadArgs
                let file_read_args = mcp_args.to_file_read_args();
                return ToolCallPayload::FileRead {
                    name: tool_name,
                    arguments: file_read_args,
                    provider_call_id,
                };
            }
        }
        "shell_command" => {
            // shell_command → Execute (already uses string command format)
            if let Ok(args) = serde_json::from_value(arguments.clone()) {
                return ToolCallPayload::Execute {
                    name: tool_name,
                    arguments: args,
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
            // Unknown Codex tool, fall through to Generic
        }
    }

    // Fallback to generic
    ToolCallPayload::Generic {
        name: tool_name,
        arguments,
        provider_call_id,
    }
}

/// Normalize Codex session records to events
/// Handles async token notifications, JSON string parsing, and exit code extraction
pub(crate) fn normalize_codex_session(
    records: Vec<CodexRecord>,
    session_id: &str,
) -> Vec<AgentEvent> {
    // Create session_id UUID from session_id string (deterministic)
    let session_id_uuid = Uuid::new_v5(&Uuid::NAMESPACE_OID, session_id.as_bytes());
    let mut builder = EventBuilder::new(session_id_uuid);
    let mut events = Vec::new();
    let mut last_seen_model: Option<String> = None;

    // Track last generation event for attaching TokenUsage (future use)
    let mut _last_generation_event_id: Option<Uuid> = None;

    // Track last seen token usage to deduplicate
    // Codex sends duplicate token_count events with same last_token_usage values
    let mut last_seen_token_usage: Option<(i32, i32, i32)> = None;

    for (row_index, record) in records.iter().enumerate() {
        // Generate base_id from session_id + row_index (deterministic)
        let base_id = format!("{}:row_{}", session_id, row_index);
        match record {
            CodexRecord::SessionMeta(_meta) => {
                // SessionMeta doesn't generate events
                // Metadata is preserved in raw field if needed
            }

            CodexRecord::EventMsg(event_msg) => {
                let timestamp = parse_timestamp(&event_msg.timestamp);
                let raw_value = attach_model_metadata(
                    serde_json::to_value(event_msg).ok(),
                    last_seen_model.as_ref(),
                );

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

                            builder.build_and_push(
                                &mut events,
                                &base_id,
                                SemanticSuffix::TokenUsage,
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
                                    }),
                                }),
                                raw_value.clone(),
                                StreamId::Main,
                            );
                        }
                    }

                    schema::EventMsgPayload::Unknown => {
                        // Skip unknown event types
                    }
                }
            }

            CodexRecord::ResponseItem(response_item) => {
                let timestamp = parse_timestamp(&response_item.timestamp);
                let raw_value = attach_model_metadata(
                    serde_json::to_value(response_item).ok(),
                    last_seen_model.as_ref(),
                );

                match &response_item.payload {
                    schema::ResponseItemPayload::Message(message) => {
                        // Extract text from content blocks
                        let text = extract_message_text(&message.content);

                        let (payload, suffix) = if message.role == "user" {
                            (
                                EventPayload::User(UserPayload { text }),
                                SemanticSuffix::User,
                            )
                        } else {
                            (
                                EventPayload::Message(MessagePayload { text }),
                                SemanticSuffix::Message,
                            )
                        };

                        let event_id = builder.build_and_push(
                            &mut events,
                            &base_id,
                            suffix,
                            timestamp,
                            payload,
                            raw_value.clone(),
                            StreamId::Main,
                        );

                        if message.role == "assistant" {
                            _last_generation_event_id = Some(event_id);
                        }
                    }

                    schema::ResponseItemPayload::Reasoning(reasoning) => {
                        // Extract text from summary blocks
                        let text = extract_reasoning_text(reasoning);

                        builder.build_and_push(
                            &mut events,
                            &base_id,
                            SemanticSuffix::Reasoning,
                            timestamp,
                            EventPayload::Reasoning(ReasoningPayload { text }),
                            raw_value.clone(),
                            StreamId::Main,
                        );
                    }

                    schema::ResponseItemPayload::FunctionCall(func_call) => {
                        // Parse JSON string arguments to Value
                        let arguments = parse_json_arguments(&func_call.arguments);

                        let event_id = builder.build_and_push(
                            &mut events,
                            &base_id,
                            SemanticSuffix::ToolCall,
                            timestamp,
                            EventPayload::ToolCall(normalize_codex_tool_call(
                                func_call.name.clone(),
                                arguments,
                                Some(func_call.call_id.clone()),
                            )),
                            raw_value.clone(),
                            StreamId::Main,
                        );

                        // Register tool call mapping
                        builder.register_tool_call(func_call.call_id.clone(), event_id);
                        _last_generation_event_id = Some(event_id);
                    }

                    schema::ResponseItemPayload::FunctionCallOutput(output) => {
                        // Extract exit code from output text
                        let exit_code = extract_exit_code(&output.output);

                        if let Some(tool_call_id) = builder.get_tool_call_uuid(&output.call_id) {
                            builder.build_and_push(
                                &mut events,
                                &base_id,
                                SemanticSuffix::ToolResult,
                                timestamp,
                                EventPayload::ToolResult(ToolResultPayload {
                                    output: output.output.clone(),
                                    tool_call_id,
                                    is_error: exit_code.map(|code| code != 0).unwrap_or(false),
                                }),
                                raw_value.clone(),
                                StreamId::Main,
                            );
                        }
                    }

                    schema::ResponseItemPayload::CustomToolCall(tool_call) => {
                        // Parse JSON string input to Value
                        let arguments = parse_json_arguments(&tool_call.input);

                        let event_id = builder.build_and_push(
                            &mut events,
                            &base_id,
                            SemanticSuffix::ToolCall,
                            timestamp,
                            EventPayload::ToolCall(normalize_codex_tool_call(
                                tool_call.name.clone(),
                                arguments,
                                Some(tool_call.call_id.clone()),
                            )),
                            raw_value.clone(),
                            StreamId::Main,
                        );

                        builder.register_tool_call(tool_call.call_id.clone(), event_id);
                        _last_generation_event_id = Some(event_id);
                    }

                    schema::ResponseItemPayload::CustomToolCallOutput(output) => {
                        let exit_code = extract_exit_code(&output.output);

                        if let Some(tool_call_id) = builder.get_tool_call_uuid(&output.call_id) {
                            builder.build_and_push(
                                &mut events,
                                &base_id,
                                SemanticSuffix::ToolResult,
                                timestamp,
                                EventPayload::ToolResult(ToolResultPayload {
                                    output: output.output.clone(),
                                    tool_call_id,
                                    is_error: exit_code.map(|code| code != 0).unwrap_or(false),
                                }),
                                raw_value.clone(),
                                StreamId::Main,
                            );
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

            CodexRecord::TurnContext(turn_context) => {
                // Track model for downstream token usage + message events
                last_seen_model = Some(turn_context.payload.model.clone());
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

    #[test]
    fn test_normalize_apply_patch_update_file() {
        let raw_patch = r#"*** Begin Patch
*** Update File: test.rs
@@
-old line
+new line
*** End Patch"#;

        let arguments = serde_json::json!({
            "raw": raw_patch
        });

        let payload = normalize_codex_tool_call(
            "apply_patch".to_string(),
            arguments,
            Some("call_123".to_string()),
        );

        match payload {
            ToolCallPayload::FileEdit {
                name,
                arguments,
                provider_call_id,
            } => {
                assert_eq!(name, "apply_patch");
                assert_eq!(arguments.file_path, "test.rs");
                assert_eq!(provider_call_id, Some("call_123".to_string()));
                // raw patch is stored in new_string
                assert!(arguments.new_string.contains("*** Update File"));
            }
            _ => panic!("Expected FileEdit variant for Update File"),
        }
    }

    #[test]
    fn test_normalize_apply_patch_add_file() {
        let raw_patch = r#"*** Begin Patch
*** Add File: docs/new_file.md
+# New File
+Content here
*** End Patch"#;

        let arguments = serde_json::json!({
            "raw": raw_patch
        });

        let payload = normalize_codex_tool_call(
            "apply_patch".to_string(),
            arguments,
            Some("call_456".to_string()),
        );

        match payload {
            ToolCallPayload::FileWrite {
                name,
                arguments,
                provider_call_id,
            } => {
                assert_eq!(name, "apply_patch");
                assert_eq!(arguments.file_path, "docs/new_file.md");
                assert_eq!(provider_call_id, Some("call_456".to_string()));
                // raw patch is stored in content
                assert!(arguments.content.contains("*** Add File"));
            }
            _ => panic!("Expected FileWrite variant for Add File"),
        }
    }

    #[test]
    fn test_normalize_shell_with_all_fields() {
        let arguments = serde_json::json!({
            "command": ["bash", "-lc", "ls"],
            "timeout_ms": 10000,
            "workdir": "/path/to/dir"
        });

        let payload =
            normalize_codex_tool_call("shell".to_string(), arguments, Some("call_789".to_string()));

        match payload {
            ToolCallPayload::Execute {
                name,
                arguments,
                provider_call_id,
            } => {
                assert_eq!(name, "shell");
                assert_eq!(arguments.command, Some("bash -lc ls".to_string()));
                assert_eq!(arguments.timeout, Some(10000));
                assert_eq!(
                    arguments.extra.get("workdir"),
                    Some(&serde_json::json!("/path/to/dir"))
                );
                assert_eq!(provider_call_id, Some("call_789".to_string()));
            }
            _ => panic!("Expected Execute variant for shell"),
        }
    }

    #[test]
    fn test_normalize_shell_minimal() {
        let arguments = serde_json::json!({
            "command": ["echo", "hello"]
        });

        let payload =
            normalize_codex_tool_call("shell".to_string(), arguments, Some("call_abc".to_string()));

        match payload {
            ToolCallPayload::Execute {
                name,
                arguments,
                provider_call_id,
            } => {
                assert_eq!(name, "shell");
                assert_eq!(arguments.command, Some("echo hello".to_string()));
                assert_eq!(arguments.timeout, None);
                assert_eq!(arguments.extra, serde_json::json!({}));
                assert_eq!(provider_call_id, Some("call_abc".to_string()));
            }
            _ => panic!("Expected Execute variant for shell"),
        }
    }

    #[test]
    fn test_normalize_read_mcp_resource() {
        let arguments = serde_json::json!({
            "server": "local",
            "uri": "/Users/zawakin/go/src/github.com/lanegrid/agtrace/AGENTS.md"
        });

        let payload = normalize_codex_tool_call(
            "read_mcp_resource".to_string(),
            arguments,
            Some("call_xyz".to_string()),
        );

        match payload {
            ToolCallPayload::FileRead {
                name,
                arguments,
                provider_call_id,
            } => {
                assert_eq!(name, "read_mcp_resource");
                assert_eq!(
                    arguments.file_path,
                    Some("/Users/zawakin/go/src/github.com/lanegrid/agtrace/AGENTS.md".to_string())
                );
                assert_eq!(arguments.path, None);
                assert_eq!(arguments.pattern, None);
                assert_eq!(
                    arguments.extra.get("server"),
                    Some(&serde_json::json!("local"))
                );
                assert_eq!(provider_call_id, Some("call_xyz".to_string()));
            }
            _ => panic!("Expected FileRead variant for read_mcp_resource"),
        }
    }

    #[test]
    fn test_normalize_shell_command() {
        let arguments = serde_json::json!({
            "command": "ls",
            "workdir": "/path/to/dir"
        });

        let payload = normalize_codex_tool_call(
            "shell_command".to_string(),
            arguments,
            Some("call_def".to_string()),
        );

        match payload {
            ToolCallPayload::Execute {
                name,
                arguments,
                provider_call_id,
            } => {
                assert_eq!(name, "shell_command");
                assert_eq!(arguments.command, Some("ls".to_string()));
                assert_eq!(
                    arguments.extra.get("workdir"),
                    Some(&serde_json::json!("/path/to/dir"))
                );
                assert_eq!(provider_call_id, Some("call_def".to_string()));
            }
            _ => panic!("Expected Execute variant for shell_command"),
        }
    }
}
