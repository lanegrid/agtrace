use agtrace_sdk::types::{AgentSession, AgentTurn, EventPayload, SessionStats, TurnStats};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

const MAX_SNIPPET_LEN: usize = 200;
const MAX_PREVIEW_LEN: usize = 300;
const MAX_PAYLOAD_LEN: usize = 500;

// ================================
// Request DTOs
// ================================

#[derive(Debug, Serialize, Deserialize)]
pub struct ListSessionsArgs {
    #[serde(default)]
    pub limit: Option<usize>,
    pub provider: Option<String>,
    pub project_hash: Option<String>,
    pub since: Option<String>,
    pub until: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetSessionDetailsArgs {
    pub session_id: String,
    #[serde(default)]
    pub include_steps: Option<bool>,
    #[serde(default)]
    pub truncate_payloads: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalyzeSessionArgs {
    pub session_id: String,
    #[serde(default)]
    pub include_failures: Option<bool>,
    #[serde(default)]
    pub include_loops: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchEventsArgs {
    pub pattern: String,
    #[serde(default)]
    pub limit: Option<usize>,
    pub provider: Option<String>,
    pub event_type: Option<String>,
    #[serde(default)]
    pub include_full_payload: Option<bool>,
}

// ================================
// Response DTOs
// ================================

#[derive(Debug, Serialize)]
pub struct ListSessionsResponse {
    pub sessions: Vec<SessionSummaryDto>,
    pub total: usize,
    pub hint: String,
}

#[derive(Debug, Serialize)]
pub struct SessionSummaryDto {
    pub id: String,
    pub project_hash: Option<String>,
    pub provider: String,
    pub start_time: Option<String>,
    pub snippet: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SessionOverviewResponse {
    pub session_id: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub stats: SessionStats,
    pub turns: Vec<TurnSummaryDto>,
    pub hint: String,
}

#[derive(Debug, Serialize)]
pub struct TurnSummaryDto {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub user_message: String,
    pub step_count: usize,
    pub stats: TurnStats,
}

#[derive(Debug, Serialize)]
pub struct SearchEventsResponse {
    pub matches: Vec<EventMatchDto>,
    pub total: usize,
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
        payload: EventPayload,
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

// ================================
// Conversion Implementations
// ================================

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
        }
    }
}

impl SessionOverviewResponse {
    pub fn from_assembled(session: AgentSession) -> Self {
        Self {
            session_id: session.session_id.to_string(),
            start_time: session.start_time,
            end_time: session.end_time,
            stats: session.stats,
            turns: session
                .turns
                .into_iter()
                .map(TurnSummaryDto::from_sdk)
                .collect(),
            hint:
                "Use get_session_details(session_id, include_steps=true) to see full step details"
                    .to_string(),
        }
    }
}

impl TurnSummaryDto {
    pub fn from_sdk(turn: AgentTurn) -> Self {
        Self {
            id: turn.id.to_string(),
            timestamp: turn.timestamp,
            user_message: truncate_string(&turn.user.content.text, MAX_SNIPPET_LEN),
            step_count: turn.steps.len(),
            stats: turn.stats,
        }
    }
}

impl EventPreviewDto {
    pub fn from_payload(payload: &EventPayload) -> Self {
        match payload {
            EventPayload::ToolCall(tc) => {
                let args_json = serde_json::to_value(tc).unwrap_or_default();
                let args = if let Some(args) = args_json.get("arguments") {
                    truncate_json_value(args, 100)
                } else {
                    serde_json::Value::Object(Default::default())
                };
                EventPreviewDto::ToolCall {
                    tool: tc.name().to_string(),
                    arguments: args,
                }
            }
            EventPayload::ToolResult(tr) => {
                let result_str = serde_json::to_string(tr).unwrap_or_default();
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
                let notif_str = serde_json::to_string(n).unwrap_or_default();
                EventPreviewDto::Text {
                    text: truncate_string(&notif_str, MAX_PREVIEW_LEN),
                }
            }
        }
    }
}

// ================================
// Utility Functions
// ================================

pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s.chars().take(max_len - 3).collect::<String>())
    } else {
        s.to_string()
    }
}

pub fn truncate_json_value(value: &serde_json::Value, max_string_len: usize) -> serde_json::Value {
    match value {
        serde_json::Value::String(s) => {
            serde_json::Value::String(truncate_string(s, max_string_len))
        }
        serde_json::Value::Array(arr) => serde_json::Value::Array(
            arr.iter()
                .take(3)
                .map(|v| truncate_json_value(v, max_string_len))
                .collect(),
        ),
        serde_json::Value::Object(obj) => serde_json::Value::Object(
            obj.iter()
                .take(5)
                .map(|(k, v)| (k.clone(), truncate_json_value(v, max_string_len)))
                .collect(),
        ),
        _ => value.clone(),
    }
}

pub fn truncate_session_payloads(value: &mut serde_json::Value) {
    if let Some(turns) = value.get_mut("turns").and_then(|v| v.as_array_mut()) {
        for turn in turns {
            if let Some(steps) = turn.get_mut("steps").and_then(|v| v.as_array_mut()) {
                for step in steps {
                    truncate_reasoning(step);
                    truncate_tool_executions(step);
                }
            }
        }
    }
}

fn truncate_reasoning(step: &mut serde_json::Value) {
    if let Some(reasoning) = step.get_mut("reasoning")
        && let Some(content) = reasoning.get_mut("content")
        && let Some(text) = content.get_mut("text").and_then(|v| v.as_str())
        && text.len() > MAX_PAYLOAD_LEN
    {
        *content.get_mut("text").unwrap() =
            serde_json::Value::String(truncate_string(text, MAX_PAYLOAD_LEN));
    }
}

fn truncate_tool_executions(step: &mut serde_json::Value) {
    if let Some(tools) = step.get_mut("tools").and_then(|v| v.as_array_mut()) {
        for tool_exec in tools {
            if let Some(result) = tool_exec.get_mut("result")
                && let Some(content) = result.get_mut("content")
                && let Some(text) = content.get_mut("content").and_then(|v| v.as_str())
                && text.len() > MAX_PAYLOAD_LEN
            {
                *content.get_mut("content").unwrap() =
                    serde_json::Value::String(truncate_string(text, MAX_PAYLOAD_LEN));
            }
        }
    }
}
