use agtrace_sdk::types::EventPayload;
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::mcp::models::common::EventType;

const MAX_PREVIEW_LEN: usize = 300;

#[derive(Debug, Serialize)]
pub struct SearchEventPreviewsViewModel {
    pub matches: Vec<EventPreviewViewModel>,
}

#[derive(Debug, Serialize)]
pub struct EventPreviewViewModel {
    pub session_id: String,
    pub event_index: usize,
    pub timestamp: DateTime<Utc>,
    pub event_type: EventType,
    pub preview: PreviewContent,
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
        use crate::mcp::models::common::{ToolSummarizer, truncate_json_value, truncate_string};

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

#[derive(Debug, Serialize)]
pub struct EventDetailsViewModel {
    pub session_id: String,
    pub event_index: usize,
    pub timestamp: DateTime<Utc>,
    pub event_type: EventType,
    pub payload: EventPayload,
}
