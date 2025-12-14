use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Agent event (v2 schema)
/// Maps 1:1 to database table row
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEvent {
    /// Unique event ID
    pub id: Uuid,

    /// Session/trace ID (groups entire conversation)
    pub trace_id: Uuid,

    /// Parent event ID in time-series chain (Linked List structure)
    /// None for root events (first User input)
    pub parent_id: Option<Uuid>,

    /// Event timestamp (UTC)
    pub timestamp: DateTime<Utc>,

    /// Event type and content (flattened enum)
    #[serde(flatten)]
    pub payload: EventPayload,

    /// Provider-specific raw data and debug information
    /// Examples: Codex "call_id", Gemini "finish_reason", etc.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

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
}

// --- Payload Definitions ---

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
pub struct ToolCallPayload {
    /// Tool name to execute
    pub name: String,
    /// Tool arguments (JSON)
    pub arguments: Value,
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
    /// Cache read input tokens (Claude, etc.)
    pub cache_read_input_tokens: Option<i32>,
    /// Reasoning output tokens (o1/Gemini, etc.)
    pub reasoning_output_tokens: Option<i32>,
}

// --- Helper Methods ---

impl AgentEvent {
    /// Check if this event is a "generation event" (can have TokenUsage children)
    pub fn is_generation_event(&self) -> bool {
        matches!(
            self.payload,
            EventPayload::ToolCall(_) | EventPayload::Message(_)
        )
    }

    /// Check if this event should be included in LLM context history
    /// (TokenUsage is excluded from context)
    pub fn is_context_event(&self) -> bool {
        !matches!(self.payload, EventPayload::TokenUsage(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialization() {
        let event = AgentEvent {
            id: Uuid::new_v4(),
            trace_id: Uuid::new_v4(),
            parent_id: None,
            timestamp: Utc::now(),
            payload: EventPayload::User(UserPayload {
                text: "Hello".to_string(),
            }),
            metadata: None,
        };

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: AgentEvent = serde_json::from_str(&json).unwrap();

        match deserialized.payload {
            EventPayload::User(payload) => assert_eq!(payload.text, "Hello"),
            _ => panic!("Wrong payload type"),
        }
    }

    #[test]
    fn test_is_generation_event() {
        let tool_call = AgentEvent {
            id: Uuid::new_v4(),
            trace_id: Uuid::new_v4(),
            parent_id: None,
            timestamp: Utc::now(),
            payload: EventPayload::ToolCall(ToolCallPayload {
                name: "bash".to_string(),
                arguments: serde_json::json!({"command": "ls"}),
            }),
            metadata: None,
        };
        assert!(tool_call.is_generation_event());

        let message = AgentEvent {
            id: Uuid::new_v4(),
            trace_id: Uuid::new_v4(),
            parent_id: None,
            timestamp: Utc::now(),
            payload: EventPayload::Message(MessagePayload {
                text: "Done".to_string(),
            }),
            metadata: None,
        };
        assert!(message.is_generation_event());

        let user = AgentEvent {
            id: Uuid::new_v4(),
            trace_id: Uuid::new_v4(),
            parent_id: None,
            timestamp: Utc::now(),
            payload: EventPayload::User(UserPayload {
                text: "Hi".to_string(),
            }),
            metadata: None,
        };
        assert!(!user.is_generation_event());
    }

    #[test]
    fn test_is_context_event() {
        let token_usage = AgentEvent {
            id: Uuid::new_v4(),
            trace_id: Uuid::new_v4(),
            parent_id: None,
            timestamp: Utc::now(),
            payload: EventPayload::TokenUsage(TokenUsagePayload {
                input_tokens: 100,
                output_tokens: 50,
                total_tokens: 150,
                details: None,
            }),
            metadata: None,
        };
        assert!(!token_usage.is_context_event());

        let user = AgentEvent {
            id: Uuid::new_v4(),
            trace_id: Uuid::new_v4(),
            parent_id: None,
            timestamp: Utc::now(),
            payload: EventPayload::User(UserPayload {
                text: "Hi".to_string(),
            }),
            metadata: None,
        };
        assert!(user.is_context_event());
    }
}
