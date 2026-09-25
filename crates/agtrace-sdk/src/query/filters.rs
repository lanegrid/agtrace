//! Filter types for session and event queries.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::types::EventPayload;

/// Provider type for filtering sessions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    /// Claude Code (Anthropic)
    ClaudeCode,
    /// GitHub Copilot Codex
    Codex,
}

impl Provider {
    pub fn as_str(&self) -> &'static str {
        match self {
            Provider::ClaudeCode => "claude_code",
            Provider::Codex => "codex",
        }
    }
}

/// Event type for filtering and classification.
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
    /// Slash command invocation
    SlashCommand,
    /// Background task queue operation
    QueueOperation,
    /// Child agent spawned
    AgentSpawn,
    /// Agent lifecycle transition
    AgentLifecycle,
    /// Inter-agent message
    AgentMessage,
    /// Context compaction
    Compaction,
    /// End of an agent turn
    TurnEnd,
    /// Model switch
    ModelChange,
    /// Context window evidence
    ContextWindowHint,
    /// Agent attribute (title, name, ...)
    AgentAttribute,
    /// Sub-action inside a tool call
    ToolSubAction,
    /// Task list / plan / goal update
    Plan,
}

impl EventType {
    /// Match against EventPayload variant.
    pub fn matches_payload(self, payload: &EventPayload) -> bool {
        self == Self::from_payload(payload)
    }

    /// Create EventType from EventPayload.
    pub fn from_payload(payload: &EventPayload) -> Self {
        match payload {
            EventPayload::ToolCall(_) => EventType::ToolCall,
            EventPayload::ToolResult(_) => EventType::ToolResult,
            EventPayload::Message(_) => EventType::Message,
            EventPayload::User(_) => EventType::User,
            EventPayload::Reasoning(_) => EventType::Reasoning,
            EventPayload::TokenUsage(_) => EventType::TokenUsage,
            EventPayload::Notification(_) => EventType::Notification,
            EventPayload::SlashCommand(_) => EventType::SlashCommand,
            EventPayload::QueueOperation(_) => EventType::QueueOperation,
            EventPayload::AgentSpawn(_) => EventType::AgentSpawn,
            EventPayload::AgentLifecycle(_) => EventType::AgentLifecycle,
            EventPayload::AgentMessage(_) => EventType::AgentMessage,
            EventPayload::Compaction(_) => EventType::Compaction,
            EventPayload::TurnEnd(_) => EventType::TurnEnd,
            EventPayload::ModelChange(_) => EventType::ModelChange,
            EventPayload::ContextWindowHint(_) => EventType::ContextWindowHint,
            EventPayload::AgentAttribute(_) => EventType::AgentAttribute,
            EventPayload::ToolSubAction(_) => EventType::ToolSubAction,
            EventPayload::Plan(_) => EventType::Plan,
        }
    }
}

/// Truncate a string to a maximum length, adding ellipsis if truncated.
pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", s.chars().take(max_len - 3).collect::<String>())
    } else {
        s.to_string()
    }
}

/// Truncate a JSON value recursively.
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
