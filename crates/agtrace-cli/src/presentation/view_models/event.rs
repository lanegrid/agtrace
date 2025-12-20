use agtrace_types::StreamId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventViewModel {
    pub id: String,
    pub session_id: String,
    pub parent_id: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub stream_id: StreamId,
    pub payload: EventPayloadViewModel,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventPayloadViewModel {
    User {
        text: String,
    },
    Reasoning {
        text: String,
    },
    ToolCall {
        name: String,
        arguments: Value,
    },
    ToolResult {
        output: String,
        is_error: bool,
    },
    Message {
        text: String,
    },
    TokenUsage {
        input: i32,
        output: i32,
        total: i32,
        cache_creation: Option<i32>,
        cache_read: Option<i32>,
    },
    Notification {
        text: String,
        level: Option<String>,
    },
}
