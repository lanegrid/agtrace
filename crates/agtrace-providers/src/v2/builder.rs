use agtrace_types::v2::*;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use uuid::Uuid;

/// EventBuilder helps convert provider raw data to v2 events
/// Maintains state for proper parent_id chain and tool_call_id mapping
pub struct EventBuilder {
    /// Current trace/session ID
    pub trace_id: Uuid,

    /// Most recent event ID in time-series chain
    /// Used as parent_id for next event
    last_event_id: Option<Uuid>,

    /// Provider tool call ID -> UUID mapping
    /// Allows O(1) lookup when creating ToolResult events
    tool_map: HashMap<String, Uuid>,
}

impl EventBuilder {
    pub fn new(trace_id: Uuid) -> Self {
        Self {
            trace_id,
            last_event_id: None,
            tool_map: HashMap::new(),
        }
    }

    /// Create a new event with proper parent_id chaining
    pub fn create_event(
        &mut self,
        timestamp: DateTime<Utc>,
        payload: EventPayload,
        metadata: Option<serde_json::Value>,
    ) -> AgentEvent {
        let id = Uuid::new_v4();
        let event = AgentEvent {
            id,
            trace_id: self.trace_id,
            parent_id: self.last_event_id,
            timestamp,
            payload,
            metadata,
        };

        // Update chain for next event
        self.last_event_id = Some(id);
        event
    }

    /// Register a tool call in the map (provider ID -> UUID)
    pub fn register_tool_call(&mut self, provider_id: String, uuid: Uuid) {
        self.tool_map.insert(provider_id, uuid);
    }

    /// Get UUID for a provider tool call ID
    pub fn get_tool_call_uuid(&self, provider_id: &str) -> Option<Uuid> {
        self.tool_map.get(provider_id).copied()
    }

    /// Reset the event chain (e.g., for new user message)
    /// Not currently used, but available if needed for future logic
    #[allow(dead_code)]
    pub fn reset_chain(&mut self) {
        self.last_event_id = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_builder_chain() {
        let trace_id = Uuid::new_v4();
        let mut builder = EventBuilder::new(trace_id);

        // First event has no parent
        let event1 = builder.create_event(
            Utc::now(),
            EventPayload::User(UserPayload {
                text: "Hello".to_string(),
            }),
            None,
        );
        assert_eq!(event1.parent_id, None);
        assert_eq!(event1.trace_id, trace_id);

        // Second event has first as parent
        let event2 = builder.create_event(
            Utc::now(),
            EventPayload::Message(MessagePayload {
                text: "Hi".to_string(),
            }),
            None,
        );
        assert_eq!(event2.parent_id, Some(event1.id));

        // Third event has second as parent
        let event3 = builder.create_event(
            Utc::now(),
            EventPayload::ToolCall(ToolCallPayload {
                name: "bash".to_string(),
                arguments: serde_json::json!({"command": "ls"}),
            }),
            None,
        );
        assert_eq!(event3.parent_id, Some(event2.id));
    }

    #[test]
    fn test_tool_map() {
        let mut builder = EventBuilder::new(Uuid::new_v4());
        let tool_uuid = Uuid::new_v4();

        builder.register_tool_call("gemini-tool-123".to_string(), tool_uuid);

        assert_eq!(
            builder.get_tool_call_uuid("gemini-tool-123"),
            Some(tool_uuid)
        );
        assert_eq!(builder.get_tool_call_uuid("nonexistent"), None);
    }
}
