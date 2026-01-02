use agtrace_sdk::types::{AgentEvent, EventPayload};
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::mcp::models::common::{
    EventType, Provider, ToolSummarizer, truncate_json_value, truncate_string,
};

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

    /// Search within specific session only (optional)
    pub session_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SearchEventPreviewsViewModel {
    pub matches: Vec<EventPreviewViewModel>,
}

impl SearchEventPreviewsViewModel {
    pub fn new(matches: Vec<EventPreviewViewModel>) -> Self {
        Self { matches }
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
                let summary = ToolSummarizer::summarize_call(tc);
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
