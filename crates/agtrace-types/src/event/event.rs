use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::payload::EventPayload;
use super::stream::StreamId;

#[cfg(test)]
use super::payload::UserPayload;

// NOTE: Schema Design Goals
//
// 1. Normalization: Abstract provider-specific quirks into unified time-series events
//    - Gemini: Unfold nested batch records into sequential events
//    - Codex: Align async token notifications and eliminate echo duplicates
//    - Claude: Extract embedded usage into independent events
//
// 2. Observability: Enable accurate cost/performance tracking
//    - Token: Sidecar pattern + incremental detection for precise billing (no double-counting)
//    - Latency: Measure turnaround time (T_req → T_res) from user perspective
//
// 3. Replayability: Reconstruct full conversation context via parent_id chain
//    - Linked-list structure ensures deterministic history recovery regardless of parallel execution
//
// 4. Separation: Distinguish time-series flow (parent_id) from logical relations (tool_call_id)
//    - Enables both "conversation replay" and "request/response mapping"
//
// NOTE: Intentional Limitations (Not Goals)
//
// - OS-level execution timestamps: Unavailable in logs; command issue time ≒ execution start
// - Tree/branch structure: Parallel tool calls are linearized in chronological/array order
// - Real-time token sync: Codex-style delayed tokens handled via eventual consistency (sidecar)
// - Gemini token breakdown: Total usage attached to final generation event (no speculation)

/// Agent event
/// Maps 1:1 to database table row
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEvent {
    /// Unique event ID
    pub id: Uuid,

    /// Session/trace ID (groups entire conversation)
    pub session_id: Uuid,

    /// Parent event ID in time-series chain (Linked List structure)
    /// None for root events (first User input)
    pub parent_id: Option<Uuid>,

    /// Event timestamp (UTC)
    pub timestamp: DateTime<Utc>,

    /// Stream identifier (main, sidechain, subagent)
    /// Enables parallel conversation streams within same session
    #[serde(default)]
    pub stream_id: StreamId,

    /// Event type and content (flattened enum)
    #[serde(flatten)]
    pub payload: EventPayload,

    /// Provider-specific raw data and debug information
    /// Examples: Codex "call_id", Gemini "finish_reason", etc.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialization() {
        let event = AgentEvent {
            id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            parent_id: None,
            timestamp: Utc::now(),
            stream_id: StreamId::Main,
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
}
