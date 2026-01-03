// MCP Common Types
//
// Design Rationale:
// - EventType/Provider enums: Type safety over string filters (prevents typos, enables validation)

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Provider type for filtering sessions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    /// Claude Code (Anthropic)
    ClaudeCode,
    /// GitHub Copilot Codex
    Codex,
    /// Google Gemini
    Gemini,
}

impl Provider {
    pub fn as_str(&self) -> &'static str {
        match self {
            Provider::ClaudeCode => "claude_code",
            Provider::Codex => "codex",
            Provider::Gemini => "gemini",
        }
    }
}

/// Event type for filtering and classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub enum EventType {
    /// Tool/function call from assistant
    ToolCall,
    /// Tool execution result
    ToolResult,
    /// Assistant message/response
    Message,
    /// User input
    User,
    /// Reasoning/thinking blocks
    Reasoning,
    /// Token usage statistics
    TokenUsage,
    /// System notification
    Notification,
}

impl EventType {
    /// Match against agtrace_sdk::types::EventPayload variant name
    pub fn matches_payload(self, payload: &agtrace_sdk::types::EventPayload) -> bool {
        use agtrace_sdk::types::EventPayload;
        matches!(
            (self, payload),
            (EventType::ToolCall, EventPayload::ToolCall(_))
                | (EventType::ToolResult, EventPayload::ToolResult(_))
                | (EventType::Message, EventPayload::Message(_))
                | (EventType::User, EventPayload::User(_))
                | (EventType::Reasoning, EventPayload::Reasoning(_))
                | (EventType::TokenUsage, EventPayload::TokenUsage(_))
                | (EventType::Notification, EventPayload::Notification(_))
        )
    }

    /// Create EventType from EventPayload
    pub fn from_payload(payload: &agtrace_sdk::types::EventPayload) -> Self {
        use agtrace_sdk::types::EventPayload;
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
}

/// Truncate a string to a maximum length, adding ellipsis if truncated
pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s.chars().take(max_len - 3).collect::<String>())
    } else {
        s.to_string()
    }
}

/// Truncate a JSON value recursively
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
