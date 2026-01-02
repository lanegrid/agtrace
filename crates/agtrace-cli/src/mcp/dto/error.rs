use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Structured error response for MCP tools
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct McpError {
    /// Machine-readable error code
    pub code: ErrorCode,
    /// Human-readable error message
    pub message: String,
    /// Additional context (e.g., matched session IDs for ambiguous prefix)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    /// Whether the operation can be retried
    pub retryable: bool,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// Session ID not found in index
    SessionNotFound,
    /// Session ID prefix matches multiple sessions
    AmbiguousSessionPrefix,
    /// Invalid session ID format
    InvalidSessionId,
    /// Event index out of bounds for session
    InvalidEventIndex,
    /// Pagination cursor is invalid or expired
    InvalidCursor,
    /// Parameter validation failed
    InvalidParameter,
    /// Search operation timed out
    SearchTimeout,
    /// Internal server error
    InternalError,
}

impl McpError {
    pub fn session_not_found(session_id: &str) -> Self {
        Self {
            code: ErrorCode::SessionNotFound,
            message: format!("Session not found: {}", session_id),
            details: Some(serde_json::json!({ "session_id": session_id })),
            retryable: false,
        }
    }

    pub fn ambiguous_prefix(prefix: &str, matches: Vec<String>) -> Self {
        Self {
            code: ErrorCode::AmbiguousSessionPrefix,
            message: format!(
                "Session ID prefix '{}' matches {} sessions. Provide more characters.",
                prefix,
                matches.len()
            ),
            details: Some(serde_json::json!({
                "prefix": prefix,
                "matches": matches,
            })),
            retryable: false,
        }
    }

    pub fn invalid_event_index(session_id: &str, index: usize, max: usize) -> Self {
        Self {
            code: ErrorCode::InvalidEventIndex,
            message: format!(
                "Event index {} out of bounds for session {} (max: {})",
                index, session_id, max
            ),
            details: Some(serde_json::json!({
                "session_id": session_id,
                "requested_index": index,
                "max_index": max,
            })),
            retryable: false,
        }
    }

    pub fn invalid_cursor(cursor: &str) -> Self {
        Self {
            code: ErrorCode::InvalidCursor,
            message: "Invalid or expired pagination cursor".to_string(),
            details: Some(serde_json::json!({ "cursor": cursor })),
            retryable: false,
        }
    }

    pub fn invalid_parameter(param_name: &str, reason: &str) -> Self {
        Self {
            code: ErrorCode::InvalidParameter,
            message: format!("Invalid parameter '{}': {}", param_name, reason),
            details: Some(serde_json::json!({
                "parameter": param_name,
                "reason": reason,
            })),
            retryable: false,
        }
    }

    pub fn internal_error(message: String) -> Self {
        Self {
            code: ErrorCode::InternalError,
            message,
            details: None,
            retryable: true,
        }
    }
}

impl From<agtrace_sdk::Error> for McpError {
    fn from(err: agtrace_sdk::Error) -> Self {
        // Convert SDK errors to MCP errors
        let message = err.to_string();

        // Try to detect specific error types from message
        if message.contains("not found") || message.contains("Not found") {
            Self::internal_error(message)
        } else {
            Self::internal_error(message)
        }
    }
}

impl std::fmt::Display for McpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for McpError {}
