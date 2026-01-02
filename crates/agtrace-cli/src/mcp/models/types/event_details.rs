use agtrace_sdk::types::EventPayload;
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::mcp::models::common::EventType;

/// Retrieve full event payload by session and index
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetEventDetailsArgs {
    /// Session ID obtained from search_event_previews response (use the 'session_id' field).
    /// Accepts 8-character prefix (e.g., "fb3cff44") or full UUID.
    /// REQUIRED: Cannot be empty.
    pub session_id: String,

    /// Zero-based event index obtained from search_event_previews response (use the 'event_index' field).
    /// REQUIRED: Must specify a valid index (0 to session event count - 1).
    pub event_index: usize,
}

#[derive(Debug, Serialize)]
pub struct EventDetailsViewModel {
    pub session_id: String,
    pub event_index: usize,
    pub timestamp: DateTime<Utc>,
    pub event_type: EventType,
    pub payload: EventPayload,
}

impl EventDetailsViewModel {
    pub fn new(
        session_id: String,
        event_index: usize,
        timestamp: DateTime<Utc>,
        payload: EventPayload,
    ) -> Self {
        let event_type = match &payload {
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
            timestamp,
            event_type,
            payload,
        }
    }
}
