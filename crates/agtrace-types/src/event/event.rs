use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::payload::EventPayload;
use crate::agent::AgentId;

#[cfg(test)]
use super::payload::UserPayload;

// NOTE: Schema Design Goals
//
// 1. Normalization: Abstract provider-specific quirks into unified time-series events
// 2. Observability: Enable accurate cost/performance tracking
// 3. Replayability: Reconstruct full conversation context via parent_id chain
//    (linked list per agent)
// 4. Separation: Distinguish time-series flow (parent_id) from logical relations (tool_call_id)
//
// One agent = one log file = one timeline. Events are ordered by their `origin`
// (file position) within an agent; timestamps are for display only.
//
// There is intentionally no per-event copy of the raw provider record: raw lines
// can be re-read on demand from the agent's file via `origin.byte_offset`.

/// Agent event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEvent {
    /// Unique event ID (UUIDv5(session_id, "{base_id}:{suffix}")).
    ///
    /// Upsert rule: two events with the same id denote the same fact; the later
    /// one replaces the earlier one.
    pub id: Uuid,

    /// Native session/thread of the file owner.
    pub session_id: Uuid,

    /// Agent (log file owner) this event belongs to.
    pub agent: AgentId,

    /// Parent event ID in the per-agent time-series chain (linked list).
    /// None for the first event of an agent.
    pub parent_id: Option<Uuid>,

    /// Event timestamp (UTC). Display only; inherited when the record has none.
    pub timestamp: DateTime<Utc>,

    /// Position of the source record in the agent's file (ordering + raw lookup).
    #[serde(default)]
    pub origin: EventOrigin,

    /// Event type and content (flattened enum)
    #[serde(flatten)]
    pub payload: EventPayload,
}

/// Position of the record that produced an event.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct EventOrigin {
    /// 0-based line number within the agent's file.
    pub line: u64,
    /// Start byte offset of that line.
    pub byte_offset: u64,
    /// Index of this event among events produced by the same line.
    pub sub: u16,
}

impl EventOrigin {
    pub fn new(line: u64, byte_offset: u64, sub: u16) -> Self {
        Self {
            line,
            byte_offset,
            sub,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialization() {
        let event = AgentEvent {
            id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            agent: AgentId::claude_session("s1"),
            parent_id: None,
            timestamp: Utc::now(),
            origin: EventOrigin::new(3, 120, 1),
            payload: EventPayload::User(UserPayload {
                text: "Hello".to_string(),
            }),
        };

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: AgentEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.agent, AgentId::claude_session("s1"));
        assert_eq!(deserialized.origin, EventOrigin::new(3, 120, 1));
        match deserialized.payload {
            EventPayload::User(payload) => assert_eq!(payload.text, "Hello"),
            _ => panic!("Wrong payload type"),
        }
    }

    #[test]
    fn origin_orders_by_file_position() {
        let a = EventOrigin::new(1, 10, 1);
        let b = EventOrigin::new(2, 20, 0);
        let c = EventOrigin::new(2, 20, 1);
        assert!(a < b && b < c);
    }
}
