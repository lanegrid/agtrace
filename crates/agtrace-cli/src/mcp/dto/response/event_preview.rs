use agtrace_sdk::types::EventPayload;
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::mcp::dto::common::{EventType, truncate_string};

const MAX_PREVIEW_LEN: usize = 300;

/// Event preview for search results
#[derive(Debug, Serialize)]
pub struct EventPreview {
    /// Session ID containing this event
    pub session_id: String,
    /// Zero-based index within session (for use with get_event_details)
    pub event_index: usize,
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// Event type
    pub event_type: EventType,
    /// Preview content (~300 chars)
    pub preview: PreviewContent,
}

/// Preview content based on event type
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum PreviewContent {
    ToolCall {
        tool: String,
        arguments: serde_json::Value,
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
                let args_json = serde_json::to_value(tc)
                    .unwrap_or_else(|e| serde_json::json!({"<error>": e.to_string()}));
                let arguments = if let Some(args) = args_json.get("arguments") {
                    super::super::common::truncate_json_value(args, 100)
                } else {
                    serde_json::Value::Object(Default::default())
                };
                PreviewContent::ToolCall { tool, arguments }
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

/// Response for search_event_previews tool
#[derive(Debug, Serialize)]
pub struct SearchEventPreviewsData {
    pub matches: Vec<EventPreview>,
}

/// Response for get_event_details tool
#[derive(Debug, Serialize)]
pub struct EventDetailsResponse {
    pub session_id: String,
    pub event_index: usize,
    pub timestamp: DateTime<Utc>,
    pub event_type: EventType,
    pub payload: EventPayload,
}

impl EventDetailsResponse {
    pub fn from_event(
        session_id: String,
        event_index: usize,
        event: &agtrace_sdk::types::AgentEvent,
    ) -> Self {
        let event_type = match &event.payload {
            EventPayload::ToolCall(_) => EventType::ToolCall,
            EventPayload::ToolResult(_) => EventType::ToolResult,
            EventPayload::Message(_) => EventType::Message,
            EventPayload::User(_) => EventType::User,
            EventPayload::Reasoning(_) => EventType::Reasoning,
            EventPayload::TokenUsage(_) => EventType::TokenUsage,
            EventPayload::Notification(_) => EventType::Notification,
        };

        Self {
            session_id,
            event_index,
            timestamp: event.timestamp,
            event_type,
            payload: event.payload.clone(),
        }
    }
}
