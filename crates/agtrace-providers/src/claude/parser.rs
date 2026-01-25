use crate::Result;
use agtrace_types::*;
use chrono::DateTime;
use std::path::Path;
use uuid::Uuid;

use crate::builder::{EventBuilder, SemanticSuffix};
use crate::claude::schema::*;

/// Generate a deterministic UUID for records without explicit uuid field
fn generate_record_uuid(session_id: &str, timestamp: &str, suffix: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    session_id.hash(&mut hasher);
    timestamp.hash(&mut hasher);
    suffix.hash(&mut hasher);
    format!("gen-{:016x}", hasher.finish())
}

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

/// Parse Claude timestamp to DateTime<Utc>
fn parse_timestamp(ts: &str) -> DateTime<chrono::Utc> {
    DateTime::parse_from_rfc3339(ts)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now())
}

/// Slash command info extracted from user message
struct SlashCommandInfo {
    name: String,
    args: Option<String>,
}

/// Extract slash command from user message text containing XML tags
/// Returns Some if <command-name> tag is found with a valid command, None otherwise
fn extract_slash_command(text: &str) -> Option<SlashCommandInfo> {
    // Look for <command-name>/foo</command-name> pattern
    let name_start = text.find("<command-name>")?;
    let name_end = text.find("</command-name>")?;

    if name_start >= name_end {
        return None;
    }

    let name = text[name_start + 14..name_end].trim().to_string();
    if name.is_empty() {
        return None;
    }

    // Valid slash commands always start with '/' (e.g., /commit, /exit, /skaffold-repo)
    // This prevents matching documentation text that mentions <command-name> tags
    if !name.starts_with('/') {
        return None;
    }

    // Look for optional <command-args>...</command-args>
    let args = if let (Some(args_start), Some(args_end)) =
        (text.find("<command-args>"), text.find("</command-args>"))
    {
        if args_start < args_end {
            let args_text = text[args_start + 14..args_end].trim();
            if !args_text.is_empty() {
                Some(args_text.to_string())
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    Some(SlashCommandInfo { name, args })
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
            ClaudeRecord::System(sys) => Some(sys.session_id.clone()),
            ClaudeRecord::Progress(prog) => Some(prog.session_id.clone()),
            ClaudeRecord::QueueOperation(queue) => Some(queue.session_id.clone()),
            ClaudeRecord::Summary(summ) => summ.session_id.clone(),
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
                            // Check for slash command pattern
                            if let Some(cmd) = extract_slash_command(text) {
                                builder.build_and_push(
                                    &mut events,
                                    &indexed_base_id,
                                    SemanticSuffix::SlashCommand,
                                    timestamp,
                                    EventPayload::SlashCommand(SlashCommandPayload {
                                        name: cmd.name,
                                        args: cmd.args,
                                    }),
                                    raw_value.clone(),
                                    stream_id.clone(),
                                );
                            } else {
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
                        }

                        UserContent::ToolResult {
                            tool_use_id,
                            content: result_content,
                            is_error,
                            agent_id,
                        } => {
                            // ToolResult in user message - map to ToolResult event
                            // Need to look up the tool_call_id from provider ID
                            if let Some(tool_call_id) = builder.get_tool_call_uuid(tool_use_id) {
                                let output = result_content
                                    .as_ref()
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();

                                // Prefer agent_id from content block, fall back to tool_use_result
                                let effective_agent_id = agent_id.clone().or_else(|| {
                                    user_record
                                        .tool_use_result
                                        .as_ref()
                                        .and_then(|r| r.agent_id.clone())
                                });

                                builder.build_and_push(
                                    &mut events,
                                    &indexed_base_id,
                                    SemanticSuffix::ToolResult,
                                    timestamp,
                                    EventPayload::ToolResult(ToolResultPayload {
                                        output,
                                        tool_call_id,
                                        is_error: *is_error,
                                        agent_id: effective_agent_id,
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
                                EventPayload::ToolCall(super::mapper::normalize_claude_tool_call(
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
                                        agent_id: None,
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
                        // Claude Token Conversion Rationale:
                        //
                        // Input mapping (verified from API spec):
                        //   cached   = cache_read_input_tokens (tokens from cache, still consume context)
                        //   uncached = input_tokens (fresh tokens, not from cache)
                        //
                        // Output mapping (current limitation):
                        //   generated = output_tokens (all output tokens)
                        //   reasoning = 0 (not yet separated)
                        //   tool      = 0 (not yet separated)
                        //
                        // Future improvement:
                        //   Claude's message.content[] has type field ("text", "thinking", "tool_use")
                        //   which allows separating output_tokens by parsing each content block.
                        //   This would enable proper reasoning/tool token accounting.
                        let cached = usage.cache_read_input_tokens.unwrap_or(0) as u64;
                        let uncached = usage.input_tokens as u64;
                        let input = TokenInput::new(cached, uncached);

                        let output = TokenOutput::new(
                            usage.output_tokens as u64, // all output as generated (for now)
                            0, // reasoning (TODO: parse content[type="thinking"])
                            0, // tool (TODO: parse content[type="tool_use"])
                        );

                        builder.build_and_push(
                            &mut events,
                            base_id,
                            SemanticSuffix::TokenUsage,
                            timestamp,
                            EventPayload::TokenUsage(TokenUsagePayload::new(input, output)),
                            raw_value.clone(),
                            stream_id.clone(),
                        );
                    }
                }
            }

            ClaudeRecord::FileHistorySnapshot(_snapshot) => {
                // Skip file snapshots for now (file system events)
            }

            ClaudeRecord::System(sys_record) => {
                let timestamp = parse_timestamp(&sys_record.timestamp);
                let raw_value = serde_json::to_value(&sys_record).ok();
                let base_id = &sys_record.uuid;
                let stream_id = determine_stream_id(sys_record.is_sidechain, &None);

                // System records with subtype "local_command" are slash commands
                if sys_record.subtype == "local_command"
                    && let Some(content) = &sys_record.content
                {
                    // Parse content as slash command (format: "/command args")
                    let (name, args) = if let Some(space_idx) = content.find(' ') {
                        (
                            content[..space_idx].to_string(),
                            Some(content[space_idx + 1..].to_string()),
                        )
                    } else {
                        (content.clone(), None)
                    };

                    builder.build_and_push(
                        &mut events,
                        base_id,
                        SemanticSuffix::SlashCommand,
                        timestamp,
                        EventPayload::SlashCommand(SlashCommandPayload { name, args }),
                        raw_value,
                        stream_id,
                    );
                }
                // Other system subtypes are skipped for now
            }

            ClaudeRecord::Progress(prog_record) => {
                let timestamp = parse_timestamp(&prog_record.timestamp);
                let raw_value = serde_json::to_value(&prog_record).ok();
                let base_id = &prog_record.uuid;
                let stream_id =
                    determine_stream_id(prog_record.is_sidechain, &prog_record.agent_id);

                // Only emit Notification for hook_progress (for debugging)
                // agent_progress is handled by separate subagent files
                // bash_progress, mcp_progress are covered by ToolCall/ToolResult
                if let ProgressData::HookProgress {
                    hook_event,
                    hook_name,
                    command,
                } = &prog_record.data
                {
                    let text = format!(
                        "Hook: {} ({})",
                        hook_name.as_deref().unwrap_or("unknown"),
                        hook_event
                    );
                    let level = if command.is_some() {
                        Some("info".to_string())
                    } else {
                        Some("debug".to_string())
                    };

                    builder.build_and_push(
                        &mut events,
                        base_id,
                        SemanticSuffix::Notification,
                        timestamp,
                        EventPayload::Notification(NotificationPayload { text, level }),
                        raw_value,
                        stream_id,
                    );
                }
                // Other progress types (agent_progress, bash_progress, mcp_progress) are skipped
            }

            ClaudeRecord::QueueOperation(queue_record) => {
                let timestamp = parse_timestamp(&queue_record.timestamp);
                let raw_value = serde_json::to_value(&queue_record).ok();
                let base_id = generate_record_uuid(
                    &queue_record.session_id,
                    &queue_record.timestamp,
                    "queue",
                );
                let stream_id = StreamId::Main;

                builder.build_and_push(
                    &mut events,
                    &base_id,
                    SemanticSuffix::QueueOperation,
                    timestamp,
                    EventPayload::QueueOperation(QueueOperationPayload {
                        operation: queue_record.operation.clone(),
                        content: queue_record.content.clone(),
                        task_id: queue_record.task_id.clone(),
                    }),
                    raw_value,
                    stream_id,
                );
            }

            ClaudeRecord::Summary(summ_record) => {
                // Summary records may not have timestamp, use current time as fallback
                let timestamp = summ_record
                    .timestamp
                    .as_ref()
                    .map(|ts| parse_timestamp(ts))
                    .unwrap_or_else(chrono::Utc::now);
                let raw_value = serde_json::to_value(&summ_record).ok();
                let base_id = summ_record.leaf_uuid.as_deref().unwrap_or("summary");
                let stream_id = StreamId::Main;

                builder.build_and_push(
                    &mut events,
                    base_id,
                    SemanticSuffix::Summary,
                    timestamp,
                    EventPayload::Summary(SummaryPayload {
                        summary: summ_record.summary.clone(),
                        leaf_uuid: summ_record.leaf_uuid.clone(),
                    }),
                    raw_value,
                    stream_id,
                );
            }

            ClaudeRecord::Unknown => {
                // Skip unknown record types
            }
        }
    }

    events
}

/// Claude session parser implementation
pub struct ClaudeParser;

impl crate::traits::SessionParser for ClaudeParser {
    fn parse_file(&self, path: &Path) -> Result<Vec<AgentEvent>> {
        super::io::normalize_claude_file(path)
    }

    fn parse_record(&self, content: &str) -> Result<Option<AgentEvent>> {
        // Claude uses JSONL format, parse as AgentEvent
        match serde_json::from_str::<AgentEvent>(content) {
            Ok(event) => Ok(Some(event)),
            Err(_) => Ok(None), // Skip malformed lines
        }
    }
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
            message: crate::claude::schema::UserMessage {
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
            tool_use_result: None,
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
                // input_tokens=100, cache_read_input_tokens=10
                // => cached=10, uncached=100, total=110
                assert_eq!(payload.input.cached, 10);
                assert_eq!(payload.input.uncached, 100);
                assert_eq!(payload.input.total(), 110);
                assert_eq!(payload.output.total(), 50);
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

    #[test]
    fn test_extract_slash_command_valid() {
        // Valid slash commands start with /
        let text =
            "<command-name>/commit</command-name>\n<command-message>commit</command-message>";
        let result = extract_slash_command(text);
        assert!(result.is_some());
        let cmd = result.unwrap();
        assert_eq!(cmd.name, "/commit");
    }

    #[test]
    fn test_extract_slash_command_with_args() {
        let text = "<command-name>/exit</command-name>\n<command-message>exit</command-message>\n<command-args>--force</command-args>";
        let result = extract_slash_command(text);
        assert!(result.is_some());
        let cmd = result.unwrap();
        assert_eq!(cmd.name, "/exit");
        assert_eq!(cmd.args, Some("--force".to_string()));
    }

    #[test]
    fn test_extract_slash_command_rejects_documentation_text() {
        // Context compaction summary text that mentions <command-name> tags
        // should NOT be parsed as a slash command
        let text = "The parser's `extract_slash_command()` function finds `<command-name>` XML tags in user messages\n- Identified the root cause: agtrace treats these as plain text";
        let result = extract_slash_command(text);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_slash_command_rejects_invalid_format() {
        // Missing closing tag
        let text = "<command-name>/commit";
        assert!(extract_slash_command(text).is_none());

        // Empty name
        let text = "<command-name></command-name>";
        assert!(extract_slash_command(text).is_none());

        // Name without leading slash
        let text = "<command-name>commit</command-name>";
        assert!(extract_slash_command(text).is_none());
    }
}
