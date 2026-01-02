mod full;
mod steps;
mod summary;
mod turns;

pub use full::SessionFullResponse;
pub use steps::SessionStepsResponse;
pub use summary::SessionSummaryResponse;
pub use turns::SessionTurnsResponse;

use agtrace_sdk::types::EventPayload;
use chrono::{DateTime, Utc};
use serde::Serialize;

use super::common::truncate_string;

const MAX_SNIPPET_LEN: usize = 200;
const MAX_PREVIEW_LEN: usize = 300;

// Re-export for other response types

#[derive(Debug, Serialize)]
pub struct ListSessionsResponse {
    pub sessions: Vec<SessionSummaryDto>,
    pub total_in_page: usize,
    pub next_cursor: Option<String>,
    pub hint: String,
}

#[derive(Debug, Serialize)]
pub struct SessionSummaryDto {
    pub id: String,
    pub project_hash: Option<String>,
    pub provider: String,
    pub start_time: Option<String>,
    pub snippet: Option<String>,
    pub turn_count: Option<usize>,
    pub duration_seconds: Option<u64>,
    pub total_tokens: Option<u64>,
}

impl SessionSummaryDto {
    pub fn from_sdk(summary: agtrace_sdk::SessionSummary) -> Self {
        Self {
            id: summary.id,
            project_hash: Some(summary.project_hash.to_string()),
            provider: summary.provider,
            start_time: summary.start_ts,
            snippet: summary
                .snippet
                .map(|s| truncate_string(&s, MAX_SNIPPET_LEN)),
            turn_count: None,       // TODO: Add to SessionSummary in index layer
            duration_seconds: None, // TODO: Add to SessionSummary in index layer
            total_tokens: None,     // TODO: Add to SessionSummary in index layer
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SearchEventsResponse {
    pub matches: Vec<EventMatchDto>,
    pub total: usize,
    pub next_cursor: Option<String>,
    pub hint: String,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum EventMatchDto {
    Snippet {
        session_id: String,
        timestamp: DateTime<Utc>,
        #[serde(rename = "type")]
        event_type: String,
        preview: EventPreviewDto,
    },
    Full {
        session_id: String,
        timestamp: DateTime<Utc>,
        #[serde(rename = "type")]
        event_type: String,
        payload: agtrace_sdk::types::EventPayload,
    },
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum EventPreviewDto {
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

impl EventPreviewDto {
    pub fn from_payload(payload: &EventPayload) -> Self {
        match payload {
            EventPayload::ToolCall(tc) => {
                let tool = tc.name().to_string();
                let args_json = serde_json::to_value(tc).unwrap_or_else(
                    |e| serde_json::json!({"<serialization_error>": e.to_string()}),
                );
                let arguments = if let Some(args) = args_json.get("arguments") {
                    super::common::truncate_json_value(args, 100)
                } else {
                    serde_json::Value::Object(Default::default())
                };
                EventPreviewDto::ToolCall { tool, arguments }
            }
            EventPayload::ToolResult(tr) => {
                let result_str = serde_json::to_string(tr)
                    .unwrap_or_else(|e| format!("<serialization_error: {}>", e));
                EventPreviewDto::ToolResult {
                    preview: truncate_string(&result_str, MAX_PREVIEW_LEN),
                }
            }
            EventPayload::User(u) => EventPreviewDto::Text {
                text: truncate_string(&u.text, MAX_PREVIEW_LEN),
            },
            EventPayload::Message(m) => EventPreviewDto::Text {
                text: truncate_string(&m.text, MAX_PREVIEW_LEN),
            },
            EventPayload::Reasoning(r) => EventPreviewDto::Text {
                text: truncate_string(&r.text, MAX_PREVIEW_LEN),
            },
            EventPayload::TokenUsage(tu) => EventPreviewDto::TokenUsage {
                input: tu.input.total(),
                output: tu.output.total(),
            },
            EventPayload::Notification(n) => {
                let notif_str = serde_json::to_string(n)
                    .unwrap_or_else(|e| format!("<serialization_error: {}>", e));
                EventPreviewDto::Text {
                    text: truncate_string(&notif_str, MAX_PREVIEW_LEN),
                }
            }
        }
    }
}
