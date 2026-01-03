use agtrace_sdk::types::{AgentEvent, EventPayload, ExecuteArgs, FileReadArgs, ToolCallPayload};
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::mcp::models::common::{EventType, Provider, truncate_json_value, truncate_string};

const MAX_PREVIEW_LEN: usize = 300;

/// Search for patterns in event payloads (returns previews only, ~300 char snippets)
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SearchEventPreviewsArgs {
    /// Search query (substring match in event JSON payloads)
    pub query: String,

    /// Maximum results per page (default: 10, max: 50)
    #[serde(default)]
    pub limit: Option<usize>,

    /// Pagination cursor from previous response's next_cursor field. Omit for first page.
    #[serde(default)]
    pub cursor: Option<String>,

    /// Filter by provider
    pub provider: Option<Provider>,

    /// Filter by event type (e.g., ToolCall, ToolResult, Message)
    pub event_type: Option<EventType>,

    /// Filter by project root path (e.g., "/Users/me/projects/my-app").
    /// Prefer this over project_hash when the agent knows the current working directory.
    /// Server will automatically resolve this to the correct project hash.
    pub project_root: Option<String>,

    /// Filter by project hash (internal ID).
    /// Use only when you have the exact hash; prefer project_root for ergonomic filtering.
    pub project_hash: Option<String>,

    /// Search within specific session only (optional)
    pub session_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SearchEventPreviewsViewModel {
    pub matches: Vec<EventPreviewViewModel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

impl SearchEventPreviewsViewModel {
    pub fn new(matches: Vec<EventPreviewViewModel>, next_cursor: Option<String>) -> Self {
        Self {
            matches,
            next_cursor,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct EventPreviewViewModel {
    pub session_id: String,
    pub event_index: usize,
    pub timestamp: DateTime<Utc>,
    pub event_type: EventType,
    pub preview: PreviewContent,
}

impl EventPreviewViewModel {
    pub fn from_event(session_id: String, event_index: usize, event: &AgentEvent) -> Self {
        let event_type = event_type_from_payload(&event.payload);
        Self {
            session_id,
            event_index,
            timestamp: event.timestamp,
            event_type,
            preview: PreviewContent::from_payload(&event.payload),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum PreviewContent {
    ToolCall {
        tool: String,
        summary: String,
        arguments_preview: serde_json::Value,
    },
    ToolResult {
        preview: String,
    },
    Text {
        text: String,
    },
    TokenUsage {
        input: u64,
        output: u64,
    },
}

impl PreviewContent {
    pub fn from_payload(payload: &EventPayload) -> Self {
        match payload {
            EventPayload::ToolCall(tc) => {
                let tool = tc.name().to_string();
                let summary = summarize_tool_call(tc);
                let args_json = serde_json::to_value(tc)
                    .unwrap_or_else(|e| serde_json::json!({"<error>": e.to_string()}));
                let arguments_preview = if let Some(args) = args_json.get("arguments") {
                    truncate_json_value(args, 100)
                } else {
                    serde_json::Value::Object(Default::default())
                };
                PreviewContent::ToolCall {
                    tool,
                    summary,
                    arguments_preview,
                }
            }
            EventPayload::ToolResult(tr) => {
                let result_str = serde_json::to_string(tr)
                    .unwrap_or_else(|e| format!("<serialization_error: {}>", e));
                PreviewContent::ToolResult {
                    preview: truncate_string(&result_str, MAX_PREVIEW_LEN),
                }
            }
            EventPayload::User(u) => PreviewContent::Text {
                text: truncate_string(&u.text, MAX_PREVIEW_LEN),
            },
            EventPayload::Message(m) => PreviewContent::Text {
                text: truncate_string(&m.text, MAX_PREVIEW_LEN),
            },
            EventPayload::Reasoning(r) => PreviewContent::Text {
                text: truncate_string(&r.text, MAX_PREVIEW_LEN),
            },
            EventPayload::TokenUsage(tu) => PreviewContent::TokenUsage {
                input: tu.input.total(),
                output: tu.output.total(),
            },
            EventPayload::Notification(n) => {
                let notif_str = serde_json::to_string(n)
                    .unwrap_or_else(|e| format!("<serialization_error: {}>", e));
                PreviewContent::Text {
                    text: truncate_string(&notif_str, MAX_PREVIEW_LEN),
                }
            }
        }
    }
}

fn event_type_from_payload(payload: &EventPayload) -> EventType {
    match payload {
        EventPayload::ToolCall(_) => EventType::ToolCall,
        EventPayload::ToolResult(_) => EventType::ToolResult,
        EventPayload::Message(_) => EventType::Message,
        EventPayload::User(_) => EventType::User,
        EventPayload::Reasoning(_) => EventType::Reasoning,
        EventPayload::TokenUsage(_) => EventType::TokenUsage,
        EventPayload::Notification(_) => EventType::Notification,
    }
}

// ============================================================================
// Tool Call Summarization Helpers
// ============================================================================

/// Summarize a tool call into a human-readable one-liner
fn summarize_tool_call(call: &ToolCallPayload) -> String {
    use ToolCallPayload::*;

    match call {
        FileRead {
            name, arguments, ..
        } => summarize_file_read(name, arguments),
        FileWrite {
            name, arguments, ..
        } => {
            let content_len = arguments.content.len();
            let lines = arguments.content.lines().count();
            let path = truncate_path(&arguments.file_path, 40);
            format!("{} {} ({} lines, {} bytes)", name, path, lines, content_len)
        }
        FileEdit {
            name, arguments, ..
        } => {
            let path = truncate_path(&arguments.file_path, 40);
            format!("{} {}", name, path)
        }
        Execute {
            name, arguments, ..
        } => summarize_execute(name, arguments),
        Search {
            name, arguments, ..
        } => {
            let args_str = serde_json::to_string(arguments).unwrap_or_default();
            let pattern_preview = truncate_string(&args_str, 30);
            format!("{} '{}'", name, pattern_preview)
        }
        Mcp {
            name, arguments, ..
        } => {
            let args_str = serde_json::to_string(arguments).unwrap_or_default();
            let preview = truncate_string(&args_str, 40);
            format!("{} {}", name, preview)
        }
        Generic {
            name, arguments, ..
        } => {
            let args_str = serde_json::to_string(arguments).unwrap_or_default();
            let preview = truncate_string(&args_str, 40);
            format!("{} {}", name, preview)
        }
    }
}

fn summarize_file_read(name: &str, args: &FileReadArgs) -> String {
    let path = if let Some(ref fp) = args.file_path {
        truncate_path(fp, 40)
    } else if let Some(ref p) = args.path {
        truncate_path(p, 40)
    } else if let Some(ref pattern) = args.pattern {
        format!("pattern '{}'", truncate_string(pattern, 30))
    } else {
        "(unknown)".to_string()
    };

    format!("{} {}", name, path)
}

fn summarize_execute(name: &str, args: &ExecuteArgs) -> String {
    let cmd_str = if let Some(ref cmd) = args.command {
        truncate_string(cmd, 40)
    } else if let Some(ref desc) = args.description {
        truncate_string(desc, 40)
    } else {
        "(unknown)".to_string()
    };

    format!("{} '{}'", name, cmd_str)
}

/// Truncate a file path, keeping the filename visible
fn truncate_path(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        return path.to_string();
    }

    // Try to keep filename visible
    if let Some(filename) = path.split('/').next_back()
        && filename.len() < max_len - 3
    {
        return format!(".../{}", filename);
    }

    truncate_string(path, max_len)
}
