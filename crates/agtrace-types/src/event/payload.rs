use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::tool::ToolCallPayload;

/// Event payload variants
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "content")]
#[serde(rename_all = "snake_case")]
pub enum EventPayload {
    /// 1. User input (Trigger)
    User(UserPayload),

    /// 2. Assistant reasoning/thinking process (Gemini thoughts, etc.)
    Reasoning(ReasoningPayload),

    /// 3. Tool execution request (Action Request)
    ///
    /// Note: TokenUsage can be attached as sidecar to this
    ToolCall(ToolCallPayload),

    /// 4. Tool execution result (Action Result)
    ToolResult(ToolResultPayload),

    /// 5. Assistant text response (Final Response)
    ///
    /// Note: TokenUsage can be attached as sidecar to this
    Message(MessagePayload),

    /// 6. Cost information (Sidecar / Leaf Node)
    ///
    /// Not included in context, used for cost calculation
    TokenUsage(TokenUsagePayload),

    /// 7. User-facing system notification (updates, alerts, status changes)
    Notification(NotificationPayload),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPayload {
    /// User input text
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningPayload {
    /// Reasoning/thinking content
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultPayload {
    /// Tool execution result (text, JSON string, error message, etc.)
    pub output: String,

    /// Logical parent (Tool Call) reference ID
    /// Separate from parent_id (time-series parent) to explicitly identify which call this result belongs to
    pub tool_call_id: Uuid,

    /// Execution success or failure
    #[serde(default)]
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagePayload {
    /// Response text
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsagePayload {
    /// Input tokens (incremental)
    pub input_tokens: i32,
    /// Output tokens (incremental)
    pub output_tokens: i32,
    /// Total tokens (incremental)
    pub total_tokens: i32,

    /// Detailed breakdown (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<TokenUsageDetails>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsageDetails {
    /// Cache creation input tokens (Claude/Gemini)
    pub cache_creation_input_tokens: Option<i32>,
    /// Cache read input tokens (Claude/Gemini)
    pub cache_read_input_tokens: Option<i32>,
    /// Reasoning output tokens (o1/Gemini)
    pub reasoning_output_tokens: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPayload {
    /// Notification message text
    pub text: String,
    /// Optional severity level (e.g., "info", "warning", "error")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
}
