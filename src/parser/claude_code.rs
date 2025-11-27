use crate::error::{Error, Result};
use crate::model::{Agent, Event, Execution, ExecutionMetrics};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Claude Code JSONL message format (as-is model capturing all fields)
#[derive(Debug, Deserialize)]
struct ClaudeCodeMessage {
    #[serde(rename = "type")]
    msg_type: String,
    #[serde(rename = "sessionId")]
    session_id: Option<String>,
    #[serde(rename = "messageId")]
    message_id: Option<String>,
    timestamp: Option<String>,
    cwd: Option<String>,
    #[serde(rename = "gitBranch")]
    git_branch: Option<String>,
    message: Option<MessageContent>,
    text: Option<String>,
    snapshot: Option<SnapshotInfo>,
    // Additional fields from actual format
    _uuid: Option<String>,
    #[serde(rename = "parentUuid")]
    _parent_uuid: Option<String>,
    #[serde(rename = "isSidechain")]
    _is_sidechain: Option<bool>,
    #[serde(rename = "userType")]
    _user_type: Option<String>,
    version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SnapshotInfo {
    #[serde(rename = "messageId")]
    message_id: Option<String>,
    timestamp: Option<String>,
    #[serde(rename = "trackedFileBackups")]
    _tracked_file_backups: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct MessageContent {
    role: Option<String>,
    #[serde(deserialize_with = "deserialize_content")]
    content: Option<Vec<ContentBlock>>,
    usage: Option<Usage>,
    model: Option<String>,
}

fn deserialize_content<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<Vec<ContentBlock>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Deserialize;

    let value = Option::<serde_json::Value>::deserialize(deserializer)?;

    match value {
        None => Ok(None),
        Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(s)) => {
            // Convert string to a text content block
            Ok(Some(vec![ContentBlock::Text { text: s }]))
        }
        Some(serde_json::Value::Array(arr)) => {
            // Try to deserialize as Vec<ContentBlock>
            let blocks: Vec<ContentBlock> = serde_json::from_value(serde_json::Value::Array(arr))
                .map_err(serde::de::Error::custom)?;
            Ok(Some(blocks))
        }
        Some(_) => {
            // For any other type (object, number, bool), return None or an empty vec
            Ok(None)
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
enum ContentBlock {
    #[serde(rename = "text", alias = "input_text")]
    Text { text: String },
    #[serde(rename = "thinking")]
    Thinking { thinking: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        name: String,
        input: serde_json::Value,
        id: Option<String>,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: Option<String>,
        #[serde(deserialize_with = "deserialize_tool_result_content")]
        content: Option<Vec<ToolResultContent>>,
    },
}

fn deserialize_tool_result_content<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<Vec<ToolResultContent>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Deserialize;

    let value = Option::<serde_json::Value>::deserialize(deserializer)?;

    match value {
        None => Ok(None),
        Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(s)) => {
            // Convert string to a ToolResultContent::String
            Ok(Some(vec![ToolResultContent::String(s)]))
        }
        Some(serde_json::Value::Array(arr)) => {
            // Try to deserialize as Vec<ToolResultContent>
            let contents: Vec<ToolResultContent> =
                serde_json::from_value(serde_json::Value::Array(arr))
                    .map_err(serde::de::Error::custom)?;
            Ok(Some(contents))
        }
        Some(_) => {
            // For any other type, return None
            Ok(None)
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
enum ToolResultContent {
    Text { text: String },
    String(String),
    // Catch-all for any other JSON value (objects, arrays, numbers, bools, null)
    Other(serde_json::Value),
}

#[derive(Debug, Deserialize)]
struct Usage {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    cache_read_input_tokens: Option<u64>,
    cache_creation_input_tokens: Option<u64>,
}

/// Parse Claude Code sessions from the default directory (~/.claude/projects)
pub fn parse_default_dir() -> Result<Vec<Execution>> {
    let home = home::home_dir()
        .ok_or_else(|| Error::Parse("Could not find home directory".to_string()))?;
    let claude_dir = home.join(".claude").join("projects");
    parse_dir(&claude_dir)
}

/// Parse Claude Code sessions from a custom directory
pub fn parse_dir(path: &Path) -> Result<Vec<Execution>> {
    if !path.exists() {
        return Err(Error::AgentDataNotFound(path.to_path_buf()));
    }

    let mut executions = Vec::new();

    // Walk through all subdirectories looking for .jsonl files
    for entry in WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("jsonl"))
    {
        match parse_jsonl_file(entry.path()) {
            Ok(exec) => executions.push(exec),
            Err(e) => {
                eprintln!("Warning: Failed to parse {}: {}", entry.path().display(), e);
            }
        }
    }

    Ok(executions)
}

/// Parse a single JSONL file into an Execution
fn parse_jsonl_file(path: &Path) -> Result<Execution> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut messages: Vec<ClaudeCodeMessage> = Vec::new();

    // Read and parse each line
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        match serde_json::from_str::<ClaudeCodeMessage>(&line) {
            Ok(msg) => messages.push(msg),
            Err(e) => {
                eprintln!("Warning: Failed to parse line in {}: {}", path.display(), e);
            }
        }
    }

    if messages.is_empty() {
        return Err(Error::Parse(format!(
            "No messages found in {}",
            path.display()
        )));
    }

    // Extract session metadata from messages (scan all messages for non-null values)
    let session_id = messages
        .iter()
        .filter_map(|m| m.session_id.clone())
        .next()
        .or_else(|| Some(format!("claude-{}", path.file_stem()?.to_str()?)))
        .unwrap_or_else(|| format!("claude-{}", chrono::Utc::now().timestamp()));

    let working_dir = messages
        .iter()
        .filter_map(|m| m.cwd.as_ref())
        .next()
        .map(|s| PathBuf::from(s))
        .unwrap_or_else(|| PathBuf::from("."));

    let git_branch = messages.iter().filter_map(|m| m.git_branch.clone()).next();

    // Parse timestamp - find the first message with a valid timestamp
    let started_at = messages
        .iter()
        .filter_map(|m| m.timestamp.as_ref())
        .filter_map(|ts| parse_timestamp(ts).ok())
        .next()
        .unwrap_or_else(chrono::Utc::now);

    let ended_at = messages
        .iter()
        .rev()
        .filter_map(|m| m.timestamp.as_ref())
        .filter_map(|ts| parse_timestamp(ts).ok())
        .next();

    // Extract model and version from messages
    let version = messages
        .iter()
        .filter_map(|m| m.version.as_ref())
        .next()
        .cloned()
        .unwrap_or_else(|| "unknown".to_string());

    let model = messages
        .iter()
        .filter_map(|m| m.message.as_ref())
        .filter_map(|msg| msg.model.clone())
        .next()
        .unwrap_or_else(|| "unknown".to_string());

    let agent = Agent::ClaudeCode { model, version };

    // Convert messages to events
    let mut events = Vec::new();
    let mut summaries = Vec::new();
    let mut input_tokens = 0u64;
    let mut output_tokens = 0u64;
    let mut cache_read_tokens = 0u64;
    let mut cache_creation_tokens = 0u64;

    let mut last_timestamp = started_at;

    for msg in messages {
        // Use the message timestamp if available, otherwise use the last known timestamp
        let timestamp = msg
            .timestamp
            .as_ref()
            .and_then(|ts| parse_timestamp(ts).ok())
            .unwrap_or(last_timestamp);

        last_timestamp = timestamp;

        match msg.msg_type.as_str() {
            "summary" => {
                if let Some(text) = msg.text {
                    summaries.push(text);
                }
            }
            "file-history-snapshot" => {
                // Extract message_id and timestamp from snapshot or top-level messageId
                let snapshot_message_id = msg
                    .snapshot
                    .as_ref()
                    .and_then(|s| s.message_id.clone())
                    .or_else(|| msg.message_id.clone())
                    .unwrap_or_default();

                let snapshot_timestamp = msg
                    .snapshot
                    .as_ref()
                    .and_then(|s| s.timestamp.as_ref())
                    .and_then(|ts| parse_timestamp(ts).ok())
                    .unwrap_or(timestamp);

                events.push(Event::FileSnapshot {
                    message_id: snapshot_message_id,
                    timestamp: snapshot_timestamp,
                });
            }
            "user" | "assistant" => {
                if let Some(message_content) = msg.message {
                    // Accumulate token usage
                    if let Some(usage) = message_content.usage {
                        input_tokens += usage.input_tokens.unwrap_or(0);
                        output_tokens += usage.output_tokens.unwrap_or(0);
                        cache_read_tokens += usage.cache_read_input_tokens.unwrap_or(0);
                        cache_creation_tokens += usage.cache_creation_input_tokens.unwrap_or(0);
                    }

                    if let Some(content_blocks) = message_content.content {
                        for block in content_blocks {
                            match block {
                                ContentBlock::Text { text } => {
                                    if message_content.role.as_deref() == Some("user") {
                                        events.push(Event::UserMessage {
                                            content: text,
                                            timestamp,
                                        });
                                    } else {
                                        events.push(Event::AssistantMessage {
                                            content: text,
                                            timestamp,
                                        });
                                    }
                                }
                                ContentBlock::Thinking { thinking } => {
                                    events.push(Event::Thinking {
                                        content: thinking,
                                        duration_ms: None,
                                        timestamp,
                                    });
                                }
                                ContentBlock::ToolUse { name, input, id } => {
                                    events.push(Event::ToolCall {
                                        name,
                                        input,
                                        call_id: id,
                                        timestamp,
                                    });
                                }
                                ContentBlock::ToolResult {
                                    tool_use_id,
                                    content,
                                } => {
                                    let output = content
                                        .map(|contents| {
                                            contents
                                                .iter()
                                                .map(|c| match c {
                                                    ToolResultContent::Text { text } => {
                                                        text.clone()
                                                    }
                                                    ToolResultContent::String(s) => s.clone(),
                                                    ToolResultContent::Other(v) => v.to_string(),
                                                })
                                                .collect::<Vec<_>>()
                                                .join("\n")
                                        })
                                        .unwrap_or_default();

                                    events.push(Event::ToolResult {
                                        call_id: tool_use_id,
                                        output,
                                        exit_code: None,
                                        duration_ms: None,
                                        timestamp,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            _ => {
                // Unknown message type, skip
            }
        }
    }

    let mut execution = Execution {
        id: session_id,
        agent,
        working_dir,
        git_branch,
        started_at,
        ended_at,
        summaries,
        events,
        metrics: ExecutionMetrics {
            input_tokens,
            output_tokens,
            cache_read_tokens,
            cache_creation_tokens,
            ..Default::default()
        },
    };

    // Compute remaining metrics from events
    execution.compute_metrics();

    // Preserve the token counts we extracted
    execution.metrics.input_tokens = input_tokens;
    execution.metrics.output_tokens = output_tokens;
    execution.metrics.cache_read_tokens = cache_read_tokens;
    execution.metrics.cache_creation_tokens = cache_creation_tokens;

    Ok(execution)
}

fn parse_timestamp(ts: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(ts)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| Error::Parse(format!("Invalid timestamp '{}': {}", ts, e)))
}
