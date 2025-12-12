use crate::model::*;
use crate::utils::project_hash_from_root;

use super::schema::*;

pub(crate) fn normalize_claude_stream(
    records: Vec<ClaudeRecord>,
    project_root_override: Option<&str>,
) -> Vec<AgentEventV1> {
    let mut events = Vec::new();
    let mut last_user_event_id: Option<String> = None;
    let mut meta_message_ids = std::collections::HashSet::new();

    let mut project_root_str: Option<String> = project_root_override.map(|s| s.to_string());
    let mut project_hash: Option<String> = project_root_str.as_deref().map(project_hash_from_root);

    for record in records {
        // Determine project_root from cwd if not overridden
        if project_root_str.is_none() {
            let cwd = match &record {
                ClaudeRecord::User(ref user) => user.cwd.as_ref(),
                ClaudeRecord::Assistant(ref asst) => asst.cwd.as_ref(),
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

        match record {
            ClaudeRecord::FileHistorySnapshot(_) => {
                // File history snapshots mark the end of a meta message chain
                meta_message_ids.clear();
                continue;
            }
            ClaudeRecord::User(user) => {
                // Track meta message IDs and their descendants
                if user.is_meta {
                    meta_message_ids.insert(user.uuid.clone());
                    continue; // Skip meta messages
                }

                // Check if this message's parent is a meta message (or descendant)
                let parent_is_meta = user
                    .parent_uuid
                    .as_ref()
                    .map(|p| meta_message_ids.contains(p))
                    .unwrap_or(false);

                // If parent is meta, this message is also considered meta-related
                if parent_is_meta {
                    meta_message_ids.insert(user.uuid.clone());
                    continue; // Skip meta descendants
                }

                // Check for tool_result content first (priority per spec v1.5)
                let has_tool_result = user
                    .message
                    .content
                    .iter()
                    .any(|c| matches!(c, UserContent::ToolResult { .. }));

                if has_tool_result {
                    // Process tool results
                    for item in &user.message.content {
                        if let UserContent::ToolResult {
                            tool_use_id,
                            content,
                            is_error,
                        } = item
                        {
                            let mut ev = AgentEventV1::new(
                                Source::ClaudeCode,
                                project_hash_val.clone(),
                                user.timestamp.clone(),
                                EventType::ToolResult,
                            );
                            ev.session_id = Some(user.session_id.clone());
                            ev.event_id = Some(format!("{}#result", tool_use_id));
                            ev.parent_event_id = last_user_event_id.clone();
                            ev.role = Some(Role::Tool); // v1.5 spec: tool_result always role=tool
                            ev.tool_call_id = Some(tool_use_id.clone());
                            ev.project_root = project_root_str.clone();

                            // Extract text from content (could be string or object)
                            // No truncation - preserve full content for analysis
                            let text_content = content.as_ref().map(|c| match c {
                                serde_json::Value::String(s) => s.clone(),
                                _ => c.to_string(),
                            });
                            ev.text = text_content.clone();

                            // Set tool_status based on is_error flag
                            ev.tool_status = Some(if *is_error {
                                ToolStatus::Error
                            } else {
                                ToolStatus::Success
                            });

                            // For Bash tool results, try to extract exit code
                            // Claude Code's Bash tool returns structured output with "Exit code: N"
                            if let Some(text) = &text_content {
                                if let Some(exit_code) = extract_exit_code_from_bash_output(text) {
                                    ev.tool_exit_code = Some(exit_code);
                                    // Non-zero exit code = error (override is_error if needed)
                                    if exit_code != 0 {
                                        ev.tool_status = Some(ToolStatus::Error);
                                    }
                                }
                            }

                            ev.raw = serde_json::to_value(item).unwrap_or(serde_json::Value::Null);
                            events.push(ev);
                        }
                    }
                    continue;
                }

                // Check for other non-text content (images, unknown)
                let has_other_content = user
                    .message
                    .content
                    .iter()
                    .any(|c| matches!(c, UserContent::Image { .. } | UserContent::Unknown));

                if has_other_content {
                    // TODO: Handle user content with images
                    continue;
                }

                // Regular user message (text only)
                let mut ev = AgentEventV1::new(
                    Source::ClaudeCode,
                    project_hash_val.clone(),
                    user.timestamp.clone(),
                    EventType::UserMessage,
                );
                ev.session_id = Some(user.session_id.clone());
                ev.event_id = Some(user.uuid.clone());
                ev.parent_event_id = None;
                ev.role = Some(Role::User);
                ev.channel = Some(Channel::Chat);
                ev.project_root = project_root_str.clone();

                let text_parts: Vec<String> = user
                    .message
                    .content
                    .iter()
                    .filter_map(|c| match c {
                        UserContent::Text { text } => Some(text.clone()),
                        _ => None,
                    })
                    .collect();
                ev.text = Some(text_parts.join("\n"));

                last_user_event_id = ev.event_id.clone();
                ev.raw = serde_json::to_value(&user).unwrap_or(serde_json::Value::Null);
                events.push(ev);
            }
            ClaudeRecord::Assistant(asst) => {
                let has_tool_content = asst.message.content.iter().any(|c| {
                    matches!(
                        c,
                        AssistantContent::ToolUse { .. } | AssistantContent::Thinking { .. }
                    )
                });

                if has_tool_content {
                    // Process each content item
                    for item in &asst.message.content {
                        match item {
                            AssistantContent::Thinking { thinking, .. } => {
                                let mut ev = AgentEventV1::new(
                                    Source::ClaudeCode,
                                    project_hash_val.clone(),
                                    asst.timestamp.clone(),
                                    EventType::Reasoning,
                                );
                                ev.session_id = Some(asst.session_id.clone());
                                ev.event_id = Some(format!("{}#thinking", asst.message.id));
                                ev.parent_event_id = last_user_event_id.clone();
                                ev.role = Some(Role::Assistant);
                                ev.channel = Some(Channel::Chat);
                                ev.project_root = project_root_str.clone();
                                ev.text = Some(thinking.clone());
                                ev.model = Some(asst.message.model.clone());
                                ev.raw =
                                    serde_json::to_value(item).unwrap_or(serde_json::Value::Null);
                                events.push(ev);
                            }
                            AssistantContent::ToolUse {
                                id, name, input, ..
                            } => {
                                let mut ev = AgentEventV1::new(
                                    Source::ClaudeCode,
                                    project_hash_val.clone(),
                                    asst.timestamp.clone(),
                                    EventType::ToolCall,
                                );
                                ev.session_id = Some(asst.session_id.clone());
                                ev.event_id = Some(id.clone());
                                ev.parent_event_id = last_user_event_id.clone();
                                ev.role = Some(Role::Assistant);
                                ev.project_root = project_root_str.clone();

                                // Normalize tool name using ToolName enum
                                let tool_name =
                                    ToolName::from_provider_name(Source::ClaudeCode, name);
                                ev.tool_name = Some(tool_name.to_string());
                                ev.channel = Some(tool_name.channel());
                                ev.tool_call_id = Some(id.clone());

                                // Extract file_path from input
                                if let Some(file_path) =
                                    input.get("file_path").and_then(|v| v.as_str())
                                {
                                    ev.file_path = Some(file_path.to_string());
                                    // Infer file_op from tool name
                                    ev.file_op = match tool_name {
                                        ToolName::Write => Some(FileOp::Write),
                                        ToolName::Read => Some(FileOp::Read),
                                        ToolName::Edit => Some(FileOp::Modify),
                                        _ => None,
                                    };
                                } else if let Some(path) =
                                    input.get("path").and_then(|v| v.as_str())
                                {
                                    // For Glob and other tools that use "path"
                                    ev.file_path = Some(path.to_string());
                                    ev.file_op = match tool_name {
                                        ToolName::Read | ToolName::Glob => Some(FileOp::Read),
                                        _ => None,
                                    };
                                }

                                let input_str = serde_json::to_string(input).unwrap_or_default();
                                ev.text = Some(input_str);
                                ev.model = Some(asst.message.model.clone());
                                ev.raw =
                                    serde_json::to_value(item).unwrap_or(serde_json::Value::Null);
                                events.push(ev);
                            }
                            AssistantContent::Text { text, .. } => {
                                let mut ev = AgentEventV1::new(
                                    Source::ClaudeCode,
                                    project_hash_val.clone(),
                                    asst.timestamp.clone(),
                                    EventType::AssistantMessage,
                                );
                                ev.session_id = Some(asst.session_id.clone());
                                ev.event_id = Some(asst.uuid.clone());
                                ev.parent_event_id = last_user_event_id.clone();
                                ev.role = Some(Role::Assistant);
                                ev.channel = Some(Channel::Chat);
                                ev.project_root = project_root_str.clone();
                                ev.text = Some(text.clone());
                                ev.model = Some(asst.message.model.clone());

                                if let Some(usage) = &asst.message.usage {
                                    ev.tokens_input = Some(usage.input_tokens as u64);
                                    ev.tokens_output = Some(usage.output_tokens as u64);
                                    ev.tokens_total =
                                        Some((usage.input_tokens + usage.output_tokens) as u64);
                                    ev.tokens_cached =
                                        usage.cache_read_input_tokens.map(|t| t as u64);
                                }

                                ev.raw =
                                    serde_json::to_value(item).unwrap_or(serde_json::Value::Null);
                                events.push(ev);
                            }
                            _ => {}
                        }
                    }
                } else {
                    // Simple assistant message with no tools
                    let mut ev = AgentEventV1::new(
                        Source::ClaudeCode,
                        project_hash_val.clone(),
                        asst.timestamp.clone(),
                        EventType::AssistantMessage,
                    );
                    ev.session_id = Some(asst.session_id.clone());
                    ev.event_id = Some(asst.uuid.clone());
                    ev.parent_event_id = last_user_event_id.clone();
                    ev.role = Some(Role::Assistant);
                    ev.channel = Some(Channel::Chat);
                    ev.project_root = project_root_str.clone();

                    let text_parts: Vec<String> = asst
                        .message
                        .content
                        .iter()
                        .filter_map(|c| match c {
                            AssistantContent::Text { text, .. } => Some(text.clone()),
                            _ => None,
                        })
                        .collect();
                    ev.text = Some(text_parts.join("\n"));
                    ev.model = Some(asst.message.model.clone());

                    if let Some(usage) = &asst.message.usage {
                        ev.tokens_input = Some(usage.input_tokens as u64);
                        ev.tokens_output = Some(usage.output_tokens as u64);
                        ev.tokens_total = Some((usage.input_tokens + usage.output_tokens) as u64);
                        ev.tokens_cached = usage.cache_read_input_tokens.map(|t| t as u64);
                    }

                    ev.raw = serde_json::to_value(&asst).unwrap_or(serde_json::Value::Null);
                    events.push(ev);
                }
            }
            ClaudeRecord::Unknown => {
                // Skip unknown records
                continue;
            }
        }
    }

    events
}

/// Extract exit code from Claude Code's Bash tool output
/// Claude Code Bash tool returns output in format: "Exit code: 0\nWall time: 0.1 seconds\nOutput:\n..."
fn extract_exit_code_from_bash_output(output: &str) -> Option<i32> {
    // Look for "Exit code: N" pattern (case insensitive)
    for line in output.lines() {
        let line_lower = line.to_lowercase();
        if line_lower.starts_with("exit code:") {
            // Extract the number after "Exit code:"
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 2 {
                let num_part = parts[1].trim();
                if let Ok(code) = num_part.parse::<i32>() {
                    return Some(code);
                }
            }
        }
    }
    None
}
