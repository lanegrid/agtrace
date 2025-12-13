use agtrace_types::project_hash_from_root;
use agtrace_types::*;

use super::schema::*;

const PROVIDER_NAME: &str = "codex";

/// Normalize Codex tool name to standard ToolName
fn normalize_tool_name(name: &str) -> ToolName {
    match name {
        "shell" | "shell_command" => ToolName::Bash,
        "apply_patch" => ToolName::Edit,
        other => ToolName::Other(other.to_string()),
    }
}

pub(crate) fn normalize_codex_stream(
    records: Vec<(CodexRecord, serde_json::Value)>,
    session_id: &str,
    project_root_override: Option<&str>,
) -> Vec<AgentEventV1> {
    let mut events = Vec::new();
    let mut last_user_event_id: Option<String> = None;
    let mut seq: u64 = 0;

    let mut project_root_str: Option<String> = project_root_override.map(|s| s.to_string());
    let mut project_hash: Option<String> = project_root_str.as_deref().map(project_hash_from_root);

    for (record, raw_value) in records {
        seq += 1;

        let ts = match &record {
            CodexRecord::SessionMeta(meta) => &meta.timestamp,
            CodexRecord::ResponseItem(response) => &response.timestamp,
            CodexRecord::EventMsg(event) => &event.timestamp,
            CodexRecord::TurnContext(turn) => &turn.timestamp,
            CodexRecord::Unknown => "",
        };

        // Extract cwd for project_root
        if project_root_str.is_none() {
            let cwd = match &record {
                CodexRecord::SessionMeta(meta) => Some(&meta.payload.cwd),
                CodexRecord::TurnContext(turn) => Some(&turn.payload.cwd),
                _ => None,
            };
            if let Some(cwd) = cwd {
                project_root_str = Some(cwd.clone());
                project_hash = Some(project_hash_from_root(cwd));
            }
        }

        let project_hash_val = project_hash
            .clone()
            .unwrap_or_else(|| "unknown".to_string());

        let mut ev = AgentEventV1::new(
            Source::new(PROVIDER_NAME),
            project_hash_val,
            ts.to_string(),
            EventType::Meta,
        );

        ev.session_id = Some(session_id.to_string());
        ev.project_root = project_root_str.clone();
        ev.event_id = Some(format!("{}#{}", ev.ts, seq));
        ev.parent_event_id = last_user_event_id.clone();

        match &record {
            CodexRecord::SessionMeta(_) => {
                ev.event_type = EventType::Meta;
                ev.role = Some(Role::System);
                ev.channel = Some(Channel::System);
            }
            CodexRecord::TurnContext(_) => {
                ev.event_type = EventType::Meta;
                ev.role = Some(Role::System);
                ev.channel = Some(Channel::System);
            }
            CodexRecord::ResponseItem(response) => {
                match &response.payload {
                    ResponseItemPayload::Message(msg) => match msg.role.as_str() {
                        "user" => {
                            ev.event_type = EventType::UserMessage;
                            ev.role = Some(Role::User);
                            ev.channel = Some(Channel::Chat);
                            ev.parent_event_id = None;
                            last_user_event_id = ev.event_id.clone();
                            ev.text = extract_message_text(msg);
                        }
                        "assistant" => {
                            ev.event_type = EventType::AssistantMessage;
                            ev.role = Some(Role::Assistant);
                            ev.channel = Some(Channel::Chat);
                            ev.text = extract_message_text(msg);
                        }
                        _ => {
                            ev.event_type = EventType::SystemMessage;
                            ev.role = Some(Role::System);
                            ev.channel = Some(Channel::System);
                            ev.text = extract_message_text(msg);
                        }
                    },
                    ResponseItemPayload::Reasoning(reasoning) => {
                        ev.event_type = EventType::Reasoning;
                        ev.role = Some(Role::Assistant);
                        ev.channel = Some(Channel::Chat);
                        let texts: Vec<String> = reasoning
                            .summary
                            .iter()
                            .filter_map(|s| match s {
                                SummaryText::SummaryText { text } => Some(text.clone()),
                                _ => None,
                            })
                            .collect();
                        if !texts.is_empty() {
                            ev.text = Some(texts.join("\n"));
                        }
                        if ev.text.is_none() {
                            ev.text = reasoning.content.clone();
                        }
                    }
                    ResponseItemPayload::FunctionCall(call) => {
                        ev.event_type = EventType::ToolCall;
                        ev.role = Some(Role::Assistant);

                        // Normalize tool name using provider-specific logic
                        let tool_name = normalize_tool_name(&call.name);
                        ev.tool_name = Some(tool_name.to_string());
                        ev.channel = Some(tool_name.channel());
                        ev.tool_call_id = Some(call.call_id.clone());

                        // Extract file_path from arguments
                        if let Ok(args_json) =
                            serde_json::from_str::<serde_json::Value>(&call.arguments)
                        {
                            if let Some(file_path) = args_json
                                .get("file_path")
                                .or(args_json.get("path"))
                                .and_then(|v| v.as_str())
                            {
                                ev.file_path = Some(file_path.to_string());
                                ev.file_op = match tool_name {
                                    ToolName::Edit => Some(FileOp::Modify),
                                    ToolName::Write => Some(FileOp::Write),
                                    ToolName::Read => Some(FileOp::Read),
                                    _ => None,
                                };
                            }
                        }

                        // For apply_patch (Edit), extract file path from patch text
                        if matches!(tool_name, ToolName::Edit) && ev.file_path.is_none() {
                            if let Some(file_path) = extract_file_path_from_patch(&call.arguments) {
                                ev.file_path = Some(file_path);
                                ev.file_op = Some(FileOp::Modify);
                            }
                        }

                        ev.text = Some(call.arguments.clone());
                    }
                    ResponseItemPayload::FunctionCallOutput(output) => {
                        ev.event_type = EventType::ToolResult;
                        ev.role = Some(Role::Tool);
                        ev.channel = Some(Channel::Terminal);
                        ev.tool_call_id = Some(output.call_id.clone());
                        ev.tool_status = Some(ToolStatus::Success);

                        // Try to extract exit code from output
                        if let Some(exit_code) = extract_exit_code(&output.output) {
                            ev.tool_exit_code = Some(exit_code);
                        }

                        ev.text = Some(output.output.clone());
                    }
                    ResponseItemPayload::CustomToolCall(call) => {
                        ev.event_type = EventType::ToolCall;
                        ev.role = Some(Role::Assistant);

                        // Normalize tool name using provider-specific logic
                        let tool_name = normalize_tool_name(&call.name);
                        ev.tool_name = Some(tool_name.to_string());
                        ev.channel = Some(tool_name.channel());
                        ev.tool_call_id = Some(call.call_id.clone());

                        // Extract file_path from input
                        if let Ok(input_json) =
                            serde_json::from_str::<serde_json::Value>(&call.input)
                        {
                            if let Some(file_path) = input_json
                                .get("file_path")
                                .or(input_json.get("path"))
                                .and_then(|v| v.as_str())
                            {
                                ev.file_path = Some(file_path.to_string());
                                ev.file_op = match tool_name {
                                    ToolName::Edit => Some(FileOp::Modify),
                                    ToolName::Write => Some(FileOp::Write),
                                    ToolName::Read => Some(FileOp::Read),
                                    _ => None,
                                };
                            }
                        }

                        // For apply_patch (Edit), extract file path from patch text
                        if matches!(tool_name, ToolName::Edit) && ev.file_path.is_none() {
                            if let Some(file_path) = extract_file_path_from_patch(&call.input) {
                                ev.file_path = Some(file_path);
                                ev.file_op = Some(FileOp::Modify);
                            }
                        }

                        ev.text = Some(call.input.clone());
                    }
                    ResponseItemPayload::CustomToolCallOutput(output) => {
                        ev.event_type = EventType::ToolResult;
                        ev.role = Some(Role::Tool);
                        ev.channel = Some(Channel::Terminal);
                        ev.tool_call_id = Some(output.call_id.clone());
                        ev.tool_status = Some(ToolStatus::Success);

                        // Try to extract exit code from output
                        if let Some(exit_code) = extract_exit_code(&output.output) {
                            ev.tool_exit_code = Some(exit_code);
                        }

                        ev.text = Some(output.output.clone());
                    }
                    ResponseItemPayload::GhostSnapshot(_) => {
                        ev.event_type = EventType::Meta;
                        ev.role = Some(Role::System);
                        ev.channel = Some(Channel::System);
                    }
                    ResponseItemPayload::Unknown => {
                        ev.event_type = EventType::Meta;
                        ev.role = Some(Role::System);
                        ev.channel = Some(Channel::System);
                    }
                }
            }
            CodexRecord::EventMsg(event) => match &event.payload {
                EventMsgPayload::UserMessage(_) => {
                    // Skip: UserMessage is already handled by ResponseItem
                    continue;
                }
                EventMsgPayload::AgentMessage(_) => {
                    // Skip: AgentMessage is already handled by ResponseItem
                    continue;
                }
                EventMsgPayload::AgentReasoning(_) => {
                    // Skip: AgentReasoning is already handled by ResponseItem reasoning
                    continue;
                }
                EventMsgPayload::TokenCount(token_count) => {
                    ev.event_type = EventType::Meta;
                    ev.role = Some(Role::System);
                    ev.channel = Some(Channel::System);
                    if let Some(info) = &token_count.info {
                        ev.tokens_input = Some(info.last_token_usage.input_tokens as u64);
                        ev.tokens_output = Some(info.last_token_usage.output_tokens as u64);
                        ev.tokens_total = Some(info.last_token_usage.total_tokens as u64);
                        ev.tokens_cached = Some(info.last_token_usage.cached_input_tokens as u64);
                        ev.tokens_thinking =
                            Some(info.last_token_usage.reasoning_output_tokens as u64);
                    }
                }
                EventMsgPayload::Unknown => {
                    ev.event_type = EventType::Meta;
                    ev.role = Some(Role::System);
                    ev.channel = Some(Channel::System);
                }
            },
            CodexRecord::Unknown => {
                ev.event_type = EventType::Meta;
                ev.role = Some(Role::System);
                ev.channel = Some(Channel::System);
            }
        }

        ev.raw = raw_value;
        events.push(ev);
    }

    events
}

fn extract_message_text(msg: &MessagePayload) -> Option<String> {
    let texts: Vec<String> = msg
        .content
        .iter()
        .filter_map(|c| match c {
            MessageContent::InputText { text } => Some(text.clone()),
            MessageContent::OutputText { text } => Some(text.clone()),
            _ => None,
        })
        .collect();
    if !texts.is_empty() {
        Some(texts.join("\n"))
    } else {
        None
    }
}

/// Extract exit code from output string (e.g., "Exit Code: 0")
fn extract_exit_code(output: &str) -> Option<i32> {
    // Look for "Exit Code: N" pattern
    if let Some(idx) = output.find("Exit Code:") {
        let rest = &output[idx + 11..]; // Skip "Exit Code: "
        let num_str: String = rest
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '-')
            .collect();
        num_str.parse::<i32>().ok()
    } else {
        None
    }
}

/// Extract file path from apply_patch text
/// Format: "*** Add File: path/to/file" or "*** Edit File: path/to/file"
fn extract_file_path_from_patch(patch_text: &str) -> Option<String> {
    // Look for "*** Add File: " or "*** Edit File: " patterns
    for prefix in ["*** Add File: ", "*** Edit File: ", "*** Modify File: "] {
        if let Some(idx) = patch_text.find(prefix) {
            let start = idx + prefix.len();
            let rest = &patch_text[start..];

            // Extract until newline or end of string
            let file_path: String = rest
                .chars()
                .take_while(|c| *c != '\n' && *c != '\r')
                .collect();

            let trimmed = file_path.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}
