use crate::model::*;
use crate::utils::*;
use anyhow::{Context, Result};
use serde_json::Value;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Parse Claude Code JSONL file and normalize to AgentEventV1
pub fn normalize_claude_file(
    path: &Path,
    project_root_override: Option<&str>,
) -> Result<Vec<AgentEventV1>> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read Claude file: {}", path.display()))?;

    let mut records: Vec<Value> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let v: Value = serde_json::from_str(line)
            .with_context(|| format!("Failed to parse JSON line: {}", line))?;
        records.push(v);
    }

    Ok(normalize_claude_stream(
        records.into_iter(),
        project_root_override,
    ))
}

pub fn normalize_claude_stream<I>(
    records: I,
    project_root_override: Option<&str>,
) -> Vec<AgentEventV1>
where
    I: IntoIterator<Item = Value>,
{
    let mut events = Vec::new();
    let mut last_user_event_id: Option<String> = None;

    let mut project_root_str: Option<String> = project_root_override.map(|s| s.to_string());
    let mut project_hash: Option<String> = project_root_str.as_deref().map(project_hash_from_root);

    for rec in records {
        // Extract common fields
        let ts = rec
            .get("timestamp")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let session_id = rec
            .get("sessionId")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Determine project_root from cwd if not overridden
        if project_root_str.is_none() {
            if let Some(cwd) = rec.get("cwd").and_then(|v| v.as_str()) {
                project_root_str = Some(cwd.to_string());
                project_hash = Some(project_hash_from_root(cwd));
            }
        }

        let project_hash_val = project_hash
            .clone()
            .unwrap_or_else(|| "unknown".to_string());

        let record_type = rec.get("type").and_then(|v| v.as_str()).unwrap_or("");

        match record_type {
            "message" => {
                if let Some(message) = rec.get("message") {
                    let role = message.get("role").and_then(|v| v.as_str());
                    let message_id = message
                        .get("id")
                        .and_then(|v| v.as_str())
                        .or_else(|| rec.get("uuid").and_then(|v| v.as_str()))
                        .map(|s| s.to_string());

                    // Priority-based detection (v1.5):
                    // Check content[] types BEFORE checking message.role
                    // This is critical because Claude wraps tool_result in role="user"
                    let content = message.get("content");
                    let content_array = content.and_then(|c| c.as_array());

                    // Check if content contains tool_use, tool_result, or thinking
                    let has_tool_content = content_array
                        .map(|arr| {
                            arr.iter().any(|item| {
                                let t = item.get("type").and_then(|v| v.as_str());
                                t == Some("tool_use")
                                    || t == Some("tool_result")
                                    || t == Some("thinking")
                            })
                        })
                        .unwrap_or(false);

                    if has_tool_content {
                        // Priority 1-3: Process tool_use, tool_result, thinking, and text
                        // This applies regardless of message.role
                        if let Some(arr) = content_array {
                            for item in arr {
                                let item_type = item.get("type").and_then(|v| v.as_str());

                                match item_type {
                                    Some("thinking") => {
                                        // Reasoning event
                                        let mut ev = AgentEventV1::new(
                                            Source::ClaudeCode,
                                            project_hash_val.clone(),
                                            ts.clone(),
                                            EventType::Reasoning,
                                        );

                                        ev.session_id = session_id.clone();
                                        ev.event_id = Some(format!(
                                            "{}#thinking",
                                            message_id.as_ref().unwrap_or(&"".to_string())
                                        ));
                                        ev.parent_event_id = last_user_event_id.clone();
                                        ev.role = Some(Role::Assistant);
                                        ev.channel = Some(Channel::Chat);
                                        ev.project_root = project_root_str.clone();

                                        ev.text = item
                                            .get("thinking")
                                            .or_else(|| item.get("text"))
                                            .and_then(|v| v.as_str())
                                            .map(|s| truncate(s, 2000));

                                        ev.model = message
                                            .get("model")
                                            .and_then(|v| v.as_str())
                                            .map(|s| s.to_string());
                                        ev.raw = item.clone();

                                        events.push(ev);
                                    }
                                    Some("tool_use") => {
                                        // Tool call event
                                        let mut ev = AgentEventV1::new(
                                            Source::ClaudeCode,
                                            project_hash_val.clone(),
                                            ts.clone(),
                                            EventType::ToolCall,
                                        );

                                        ev.session_id = session_id.clone();
                                        ev.event_id = item
                                            .get("id")
                                            .and_then(|v| v.as_str())
                                            .map(|s| s.to_string());
                                        ev.parent_event_id = last_user_event_id.clone();
                                        ev.role = Some(Role::Assistant);
                                        ev.project_root = project_root_str.clone();

                                        ev.tool_name = item
                                            .get("name")
                                            .and_then(|v| v.as_str())
                                            .map(|s| s.to_string());
                                        ev.tool_call_id = ev.event_id.clone();

                                        // Determine channel based on tool name
                                        ev.channel = match ev.tool_name.as_deref() {
                                            Some("Bash") => Some(Channel::Terminal),
                                            Some("Edit") | Some("Write") => Some(Channel::Editor),
                                            Some("Read") | Some("Glob") => {
                                                Some(Channel::Filesystem)
                                            }
                                            _ => Some(Channel::Chat),
                                        };

                                        // Summarize input
                                        if let Some(input) = item.get("input") {
                                            let input_str =
                                                serde_json::to_string(input).unwrap_or_default();
                                            ev.text = Some(truncate(&input_str, 500));
                                        }

                                        ev.model = message
                                            .get("model")
                                            .and_then(|v| v.as_str())
                                            .map(|s| s.to_string());
                                        ev.raw = item.clone();

                                        events.push(ev);
                                    }
                                    Some("tool_result") => {
                                        // Tool result event
                                        // v1.5: role is always "tool", even if message.role is "user"
                                        let mut ev = AgentEventV1::new(
                                            Source::ClaudeCode,
                                            project_hash_val.clone(),
                                            ts.clone(),
                                            EventType::ToolResult,
                                        );

                                        ev.session_id = session_id.clone();
                                        ev.event_id = Some(format!(
                                            "{}#result",
                                            item.get("tool_use_id")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or("")
                                        ));
                                        ev.parent_event_id = last_user_event_id.clone();
                                        ev.role = Some(Role::Tool); // v1.5: tool_result role is Tool
                                        ev.channel = Some(Channel::Terminal);
                                        ev.project_root = project_root_str.clone();

                                        ev.tool_call_id = item
                                            .get("tool_use_id")
                                            .and_then(|v| v.as_str())
                                            .map(|s| s.to_string());

                                        // Extract content/output
                                        if let Some(content) = item.get("content") {
                                            if let Some(s) = content.as_str() {
                                                ev.text = Some(truncate(s, 1000));
                                            }
                                        }

                                        ev.raw = item.clone();
                                        events.push(ev);
                                    }
                                    Some("text") => {
                                        // Assistant text message (only create if role is assistant)
                                        if role == Some("assistant") {
                                            let mut ev = AgentEventV1::new(
                                                Source::ClaudeCode,
                                                project_hash_val.clone(),
                                                ts.clone(),
                                                EventType::AssistantMessage,
                                            );

                                            ev.session_id = session_id.clone();
                                            ev.event_id = message_id.clone();
                                            ev.parent_event_id = last_user_event_id.clone();
                                            ev.role = Some(Role::Assistant);
                                            ev.channel = Some(Channel::Chat);
                                            ev.project_root = project_root_str.clone();

                                            ev.text = item
                                                .get("text")
                                                .and_then(|v| v.as_str())
                                                .map(|s| s.to_string());

                                            ev.model = message
                                                .get("model")
                                                .and_then(|v| v.as_str())
                                                .map(|s| s.to_string());

                                            // Extract token usage
                                            if let Some(usage) = message.get("usage") {
                                                ev.tokens_input = usage
                                                    .get("input_tokens")
                                                    .and_then(|v| v.as_u64());
                                                ev.tokens_output = usage
                                                    .get("output_tokens")
                                                    .and_then(|v| v.as_u64());
                                                ev.tokens_cached = usage
                                                    .get("cache_read_input_tokens")
                                                    .and_then(|v| v.as_u64());
                                            }

                                            ev.raw = rec.clone();
                                            events.push(ev);
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    } else if role == Some("user") {
                        // Priority 4: Normal user message (no tool content)
                        let mut ev = AgentEventV1::new(
                            Source::ClaudeCode,
                            project_hash_val.clone(),
                            ts.clone(),
                            EventType::UserMessage,
                        );

                        ev.session_id = session_id.clone();
                        ev.event_id = message_id.clone();
                        ev.parent_event_id = None;
                        ev.role = Some(Role::User);
                        ev.channel = Some(Channel::Chat);
                        ev.project_root = project_root_str.clone();

                        // Extract text from content
                        if let Some(content) = message.get("content") {
                            if let Some(s) = content.as_str() {
                                ev.text = Some(s.to_string());
                            } else if let Some(arr) = content.as_array() {
                                let texts: Vec<String> = arr
                                    .iter()
                                    .filter_map(|c| c.get("text").and_then(|v| v.as_str()))
                                    .map(|s| s.to_string())
                                    .collect();
                                if !texts.is_empty() {
                                    ev.text = Some(texts.join("\n"));
                                }
                            }
                        }

                        ev.model = message
                            .get("model")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        ev.raw = rec.clone();

                        last_user_event_id = ev.event_id.clone();
                        events.push(ev);
                    } else if role == Some("assistant") {
                        // Priority 4: Normal assistant message (no tool content)
                        // Only simple text messages reach here
                        let content = message.get("content");

                        if let Some(text) = content.and_then(|c| c.as_str()) {
                            // Simple text assistant message
                            let mut ev = AgentEventV1::new(
                                Source::ClaudeCode,
                                project_hash_val.clone(),
                                ts.clone(),
                                EventType::AssistantMessage,
                            );

                            ev.session_id = session_id.clone();
                            ev.event_id = message_id.clone();
                            ev.parent_event_id = last_user_event_id.clone();
                            ev.role = Some(Role::Assistant);
                            ev.channel = Some(Channel::Chat);
                            ev.project_root = project_root_str.clone();
                            ev.text = Some(text.to_string());

                            ev.model = message
                                .get("model")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());

                            if let Some(usage) = message.get("usage") {
                                ev.tokens_input =
                                    usage.get("input_tokens").and_then(|v| v.as_u64());
                                ev.tokens_output =
                                    usage.get("output_tokens").and_then(|v| v.as_u64());
                            }

                            ev.raw = rec.clone();
                            events.push(ev);
                        }
                    }
                }
            }
            "user" => {
                // Legacy format: type "user" with message field
                // v1.5: Check for tool_result in content FIRST (same priority as "message" type)
                if let Some(message) = rec.get("message") {
                    let content = message.get("content");
                    let content_array = content.and_then(|c| c.as_array());

                    // Check if content contains tool_result
                    let has_tool_result = content_array
                        .map(|arr| {
                            arr.iter().any(|item| {
                                item.get("type").and_then(|v| v.as_str()) == Some("tool_result")
                            })
                        })
                        .unwrap_or(false);

                    let message_id = rec
                        .get("uuid")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    if has_tool_result {
                        // Priority 1: Process as tool_result, not user_message
                        if let Some(arr) = content_array {
                            for item in arr {
                                if item.get("type").and_then(|v| v.as_str()) == Some("tool_result") {
                                    let mut ev = AgentEventV1::new(
                                        Source::ClaudeCode,
                                        project_hash_val.clone(),
                                        ts.clone(),
                                        EventType::ToolResult,
                                    );

                                    ev.session_id = session_id.clone();
                                    ev.event_id = Some(format!(
                                        "{}#result",
                                        item.get("tool_use_id")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                    ));
                                    ev.parent_event_id = last_user_event_id.clone();
                                    ev.role = Some(Role::Tool); // v1.5: override role
                                    ev.channel = Some(Channel::Terminal);
                                    ev.project_root = project_root_str.clone();

                                    ev.tool_call_id = item
                                        .get("tool_use_id")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string());

                                    // Extract content/output
                                    if let Some(content) = item.get("content") {
                                        if let Some(s) = content.as_str() {
                                            ev.text = Some(truncate(s, 1000));
                                        }
                                    }

                                    ev.raw = rec.clone();
                                    events.push(ev);
                                }
                            }
                        }
                    } else {
                        // Priority 2: Normal user message
                        let mut ev = AgentEventV1::new(
                            Source::ClaudeCode,
                            project_hash_val.clone(),
                            ts.clone(),
                            EventType::UserMessage,
                        );

                        ev.session_id = session_id.clone();
                        ev.event_id = message_id.clone();
                        ev.parent_event_id = None;
                        ev.role = Some(Role::User);
                        ev.channel = Some(Channel::Chat);
                        ev.project_root = project_root_str.clone();

                        // Extract text from content
                        if let Some(content) = message.get("content") {
                            if let Some(s) = content.as_str() {
                                ev.text = Some(s.to_string());
                            } else if let Some(arr) = content.as_array() {
                                let texts: Vec<String> = arr
                                    .iter()
                                    .filter_map(|c| c.get("text").and_then(|v| v.as_str()))
                                    .map(|s| s.to_string())
                                    .collect();
                                if !texts.is_empty() {
                                    ev.text = Some(texts.join("\n"));
                                }
                            }
                        }

                        ev.raw = rec.clone();
                        last_user_event_id = ev.event_id.clone();
                        events.push(ev);
                    }
                }
            }
            "assistant" => {
                // Legacy format: type "assistant" with message field
                if let Some(message) = rec.get("message") {
                    let message_id = message
                        .get("id")
                        .and_then(|v| v.as_str())
                        .or_else(|| rec.get("uuid").and_then(|v| v.as_str()))
                        .map(|s| s.to_string());

                    let content = message.get("content");

                    if let Some(arr) = content.and_then(|c| c.as_array()) {
                        for item in arr {
                            let item_type = item.get("type").and_then(|v| v.as_str());

                            match item_type {
                                Some("thinking") => {
                                    let mut ev = AgentEventV1::new(
                                        Source::ClaudeCode,
                                        project_hash_val.clone(),
                                        ts.clone(),
                                        EventType::Reasoning,
                                    );

                                    ev.session_id = session_id.clone();
                                    ev.event_id = Some(format!(
                                        "{}#thinking",
                                        message_id.as_ref().unwrap_or(&"".to_string())
                                    ));
                                    ev.parent_event_id = last_user_event_id.clone();
                                    ev.role = Some(Role::Assistant);
                                    ev.channel = Some(Channel::Chat);
                                    ev.project_root = project_root_str.clone();

                                    ev.text = item
                                        .get("thinking")
                                        .or_else(|| item.get("text"))
                                        .and_then(|v| v.as_str())
                                        .map(|s| truncate(s, 2000));

                                    ev.model = message
                                        .get("model")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string());
                                    ev.raw = item.clone();

                                    events.push(ev);
                                }
                                Some("tool_use") => {
                                    let mut ev = AgentEventV1::new(
                                        Source::ClaudeCode,
                                        project_hash_val.clone(),
                                        ts.clone(),
                                        EventType::ToolCall,
                                    );

                                    ev.session_id = session_id.clone();
                                    ev.event_id = item
                                        .get("id")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string());
                                    ev.parent_event_id = last_user_event_id.clone();
                                    ev.role = Some(Role::Assistant);
                                    ev.project_root = project_root_str.clone();

                                    ev.tool_name = item
                                        .get("name")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string());
                                    ev.tool_call_id = ev.event_id.clone();

                                    ev.channel = match ev.tool_name.as_deref() {
                                        Some("Bash") => Some(Channel::Terminal),
                                        Some("Edit") | Some("Write") => Some(Channel::Editor),
                                        Some("Read") | Some("Glob") => Some(Channel::Filesystem),
                                        _ => Some(Channel::Chat),
                                    };

                                    if let Some(input) = item.get("input") {
                                        let input_str =
                                            serde_json::to_string(input).unwrap_or_default();
                                        ev.text = Some(truncate(&input_str, 500));
                                    }

                                    ev.model = message
                                        .get("model")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string());
                                    ev.raw = item.clone();

                                    events.push(ev);
                                }
                                Some("tool_result") => {
                                    let mut ev = AgentEventV1::new(
                                        Source::ClaudeCode,
                                        project_hash_val.clone(),
                                        ts.clone(),
                                        EventType::ToolResult,
                                    );

                                    ev.session_id = session_id.clone();
                                    ev.event_id = Some(format!(
                                        "{}#result",
                                        item.get("tool_use_id")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                    ));
                                    ev.parent_event_id = last_user_event_id.clone();
                                    ev.role = Some(Role::Tool); // v1.5: tool_result role is Tool
                                    ev.channel = Some(Channel::Terminal);
                                    ev.project_root = project_root_str.clone();

                                    ev.tool_call_id = item
                                        .get("tool_use_id")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string());

                                    if let Some(content) = item.get("content") {
                                        if let Some(s) = content.as_str() {
                                            ev.text = Some(truncate(s, 1000));
                                        }
                                    }

                                    ev.raw = item.clone();
                                    events.push(ev);
                                }
                                Some("text") => {
                                    let mut ev = AgentEventV1::new(
                                        Source::ClaudeCode,
                                        project_hash_val.clone(),
                                        ts.clone(),
                                        EventType::AssistantMessage,
                                    );

                                    ev.session_id = session_id.clone();
                                    ev.event_id = message_id.clone();
                                    ev.parent_event_id = last_user_event_id.clone();
                                    ev.role = Some(Role::Assistant);
                                    ev.channel = Some(Channel::Chat);
                                    ev.project_root = project_root_str.clone();

                                    ev.text = item
                                        .get("text")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string());

                                    ev.model = message
                                        .get("model")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string());

                                    if let Some(usage) = message.get("usage") {
                                        ev.tokens_input =
                                            usage.get("input_tokens").and_then(|v| v.as_u64());
                                        ev.tokens_output =
                                            usage.get("output_tokens").and_then(|v| v.as_u64());
                                        ev.tokens_cached = usage
                                            .get("cache_read_input_tokens")
                                            .and_then(|v| v.as_u64());
                                    }

                                    ev.raw = rec.clone();
                                    events.push(ev);
                                }
                                _ => {}
                            }
                        }
                    } else if let Some(text) = content.and_then(|c| c.as_str()) {
                        let mut ev = AgentEventV1::new(
                            Source::ClaudeCode,
                            project_hash_val.clone(),
                            ts.clone(),
                            EventType::AssistantMessage,
                        );

                        ev.session_id = session_id.clone();
                        ev.event_id = message_id.clone();
                        ev.parent_event_id = last_user_event_id.clone();
                        ev.role = Some(Role::Assistant);
                        ev.channel = Some(Channel::Chat);
                        ev.project_root = project_root_str.clone();
                        ev.text = Some(text.to_string());

                        ev.model = message
                            .get("model")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());

                        if let Some(usage) = message.get("usage") {
                            ev.tokens_input = usage.get("input_tokens").and_then(|v| v.as_u64());
                            ev.tokens_output = usage.get("output_tokens").and_then(|v| v.as_u64());
                        }

                        ev.raw = rec.clone();
                        events.push(ev);
                    }
                }
            }
            "tool_result" => {
                // Standalone tool result
                if let Some(result) = rec.get("toolUseResult") {
                    let mut ev = AgentEventV1::new(
                        Source::ClaudeCode,
                        project_hash_val.clone(),
                        ts.clone(),
                        EventType::ToolResult,
                    );

                    ev.session_id = session_id.clone();
                    ev.parent_event_id = last_user_event_id.clone();
                    ev.role = Some(Role::Tool); // v1.5: tool_result role is Tool
                    ev.channel = Some(Channel::Terminal);
                    ev.project_root = project_root_str.clone();

                    ev.tool_call_id = result
                        .get("tool_use_id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    ev.event_id = Some(format!(
                        "{}#result",
                        ev.tool_call_id.as_ref().unwrap_or(&"".to_string())
                    ));

                    // Status
                    if let Some(status) = result.get("status").and_then(|v| v.as_str()) {
                        ev.tool_status = Some(match status {
                            "completed" => ToolStatus::Success,
                            _ => ToolStatus::Unknown,
                        });
                    }

                    // Output
                    if let Some(output) = result.get("output").and_then(|v| v.as_str()) {
                        ev.text = Some(truncate(output, 1000));
                    }

                    // File path
                    ev.file_path = result
                        .get("filePath")
                        .or_else(|| result.get("file").and_then(|f| f.get("filePath")))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    ev.raw = rec.clone();
                    events.push(ev);
                }
            }
            _ => {
                // Other types (summary, file-history-snapshot, etc.) can be added here
            }
        }
    }

    events
}

/// Extract cwd from a Claude session file by reading the first few lines
/// Returns None if cwd cannot be determined
pub fn extract_cwd_from_claude_file(path: &Path) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    let reader = BufReader::new(file);

    // Read first 10 lines to find cwd
    for line in reader.lines().take(10) {
        if let Ok(line) = line {
            if let Ok(json) = serde_json::from_str::<Value>(&line) {
                if let Some(cwd) = json.get("cwd").and_then(|v| v.as_str()) {
                    return Some(cwd.to_string());
                }
            }
        }
    }

    None
}

pub struct ClaudeProvider;

impl ClaudeProvider {
    pub fn new() -> Self {
        Self
    }
}

impl super::LogProvider for ClaudeProvider {
    fn name(&self) -> &str {
        "claude"
    }

    fn can_handle(&self, path: &Path) -> bool {
        if !path.is_file() {
            return false;
        }

        if !path.extension().map_or(false, |e| e == "jsonl") {
            return false;
        }

        true
    }

    fn normalize_file(&self, path: &Path, context: &super::ImportContext) -> Result<Vec<AgentEventV1>> {
        normalize_claude_file(path, context.project_root_override.as_deref())
    }

    fn belongs_to_project(&self, path: &Path, target_project_root: &Path) -> bool {
        if let Some(session_cwd) = extract_cwd_from_claude_file(path) {
            let session_cwd_path = Path::new(&session_cwd);
            paths_equal(target_project_root, session_cwd_path)
        } else {
            false
        }
    }

    fn get_search_root(&self, log_root: &Path, target_project_root: &Path) -> Option<std::path::PathBuf> {
        let dir_name = encode_claude_project_dir(target_project_root);
        let project_specific_root = log_root.join(dir_name);
        if project_specific_root.exists() && project_specific_root.is_dir() {
            Some(project_specific_root)
        } else {
            None
        }
    }
}
