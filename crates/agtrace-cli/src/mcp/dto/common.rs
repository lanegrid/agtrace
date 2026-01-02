// MCP Common Types
//
// Design Rationale:
// - EventType/Provider enums: Type safety over string filters (prevents typos, enables validation)
// - McpResponse wrapper: Consistent pagination structure across list operations
// - PaginationMeta: MCP 2024-11-05 spec (cursor-based, not offset-based)

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
}

/// Detail level for session responses
/// Pagination metadata for list responses
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PaginationMeta {
    /// Number of items in current page
    pub total_in_page: usize,
    /// Opaque cursor for next page (null if this is the last page)
    pub next_cursor: Option<String>,
    /// Quick check if more results exist
    pub has_more: bool,
}

/// Standardized response wrapper for MCP tools
#[derive(Debug, Serialize, JsonSchema)]
pub struct McpResponse<T> {
    /// Response data
    pub data: T,
    /// Pagination info (for list operations)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pagination: Option<PaginationMeta>,
    /// Usage hint for next steps
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

/// Response metadata for size tracking and pagination
///
/// Included in all MCP tool responses to help LLMs manage context window constraints.
/// Based on MCP best practices for progressive disclosure and token budgeting.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ResponseMeta {
    /// Actual JSON response size in bytes
    pub bytes: usize,

    /// Estimated token count (bytes / 4, conservative estimate for JSON)
    pub estimated_tokens: usize,

    /// Whether more data is available via pagination
    pub has_more: bool,

    /// Opaque cursor for next page (null if no more data)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,

    /// Total number of items across all pages (if known)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_items: Option<usize>,

    /// Number of items returned in this response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub returned_count: Option<usize>,
}

impl ResponseMeta {
    /// Create metadata from serialized response bytes
    pub fn from_bytes(bytes: usize) -> Self {
        Self {
            bytes,
            estimated_tokens: bytes / 4, // Conservative estimate for JSON
            has_more: false,
            next_cursor: None,
            total_items: None,
            returned_count: None,
        }
    }

    /// Create metadata with pagination info
    pub fn with_pagination(
        bytes: usize,
        next_cursor: Option<String>,
        returned_count: usize,
        total_items: Option<usize>,
    ) -> Self {
        Self {
            bytes,
            estimated_tokens: bytes / 4,
            has_more: next_cursor.is_some(),
            next_cursor,
            total_items,
            returned_count: Some(returned_count),
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
