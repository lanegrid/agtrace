use agtrace_types::*;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use uuid::Uuid;

/// Semantic suffix for deterministic UUID generation
/// Represents the "why" behind each event creation
#[derive(Debug, Clone, Copy)]
pub enum SemanticSuffix {
    User,
    Reasoning,
    Message,
    ToolCall,
    ToolResult,
    TokenUsage,
    Notification,
}

impl SemanticSuffix {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Reasoning => "reasoning",
            Self::Message => "message",
            Self::ToolCall => "call",
            Self::ToolResult => "result",
            Self::TokenUsage => "usage",
            Self::Notification => "notify",
        }
    }
}

/// EventBuilder helps convert provider raw data to events
/// Maintains state for proper parent_id chain and tool_call_id mapping
pub struct EventBuilder {
    /// Current trace/session ID
    pub trace_id: Uuid,

    /// Most recent event ID per stream in time-series chain
    /// Maps stream_id -> latest event UUID for that stream
    /// Enables independent parent chains for main/sidechain/subagent streams
    stream_tips: HashMap<StreamId, Uuid>,

    /// Provider tool call ID -> UUID mapping
    /// Allows O(1) lookup when creating ToolResult events
    tool_map: HashMap<String, Uuid>,
}

impl EventBuilder {
    pub fn new(trace_id: Uuid) -> Self {
        Self {
            trace_id,
            stream_tips: HashMap::new(),
            tool_map: HashMap::new(),
        }
    }

    /// Create and push event with deterministic UUID generation
    /// Uses UUID v5 with trace_id as namespace and "base_id:suffix" as name
    /// Returns the generated event ID
    #[allow(clippy::too_many_arguments)]
    pub fn build_and_push(
        &mut self,
        events: &mut Vec<AgentEvent>,
        base_id: &str,
        suffix: SemanticSuffix,
        timestamp: DateTime<Utc>,
        payload: EventPayload,
        metadata: Option<serde_json::Value>,
        stream_id: StreamId,
    ) -> Uuid {
        // Generate deterministic UUID: trace_id namespace + "base_id:suffix" name
        let name = format!("{}:{}", base_id, suffix.as_str());
        let id = Uuid::new_v5(&self.trace_id, name.as_bytes());

        // Get parent_id from stream-specific tip
        let parent_id = self.stream_tips.get(&stream_id).copied();

        let event = AgentEvent {
            id,
            trace_id: self.trace_id,
            parent_id,
            timestamp,
            stream_id: stream_id.clone(),
            payload,
            metadata,
        };

        let event_id = event.id;
        events.push(event);

        // Update stream tip
        self.stream_tips.insert(stream_id, event_id);
        event_id
    }

    /// Register a tool call in the map (provider ID -> UUID)
    pub fn register_tool_call(&mut self, provider_id: String, uuid: Uuid) {
        self.tool_map.insert(provider_id, uuid);
    }

    /// Get UUID for a provider tool call ID
    pub fn get_tool_call_uuid(&self, provider_id: &str) -> Option<Uuid> {
        self.tool_map.get(provider_id).copied()
    }

    /// Reset the event chain for a specific stream
    /// Not currently used, but available if needed for future logic
    #[allow(dead_code)]
    pub fn reset_stream(&mut self, stream_id: &StreamId) {
        self.stream_tips.remove(stream_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_builder_chain() {
        let trace_id = Uuid::new_v4();
        let mut builder = EventBuilder::new(trace_id);
        let mut events = Vec::new();

        // First event has no parent
        let event1_id = builder.build_and_push(
            &mut events,
            "test-id-1",
            SemanticSuffix::User,
            Utc::now(),
            EventPayload::User(UserPayload {
                text: "Hello".to_string(),
            }),
            None,
            StreamId::Main,
        );
        assert_eq!(events[0].parent_id, None);
        assert_eq!(events[0].trace_id, trace_id);
        assert_eq!(events[0].stream_id, StreamId::Main);

        // Second event has first as parent
        let event2_id = builder.build_and_push(
            &mut events,
            "test-id-2",
            SemanticSuffix::Message,
            Utc::now(),
            EventPayload::Message(MessagePayload {
                text: "Hi".to_string(),
            }),
            None,
            StreamId::Main,
        );
        assert_eq!(events[1].parent_id, Some(event1_id));

        // Third event has second as parent
        builder.build_and_push(
            &mut events,
            "test-id-3",
            SemanticSuffix::ToolCall,
            Utc::now(),
            EventPayload::ToolCall(ToolCallPayload {
                name: "bash".to_string(),
                arguments: serde_json::json!({"command": "ls"}),
                provider_call_id: Some("call_123".to_string()),
            }),
            None,
            StreamId::Main,
        );
        assert_eq!(events[2].parent_id, Some(event2_id));
    }

    #[test]
    fn test_multi_stream_chains() {
        let trace_id = Uuid::new_v4();
        let mut builder = EventBuilder::new(trace_id);
        let mut events = Vec::new();

        // Main stream events
        let main1_id = builder.build_and_push(
            &mut events,
            "main-1",
            SemanticSuffix::User,
            Utc::now(),
            EventPayload::User(UserPayload {
                text: "Main".to_string(),
            }),
            None,
            StreamId::Main,
        );

        // Sidechain stream events
        let _side1_id = builder.build_and_push(
            &mut events,
            "side-1",
            SemanticSuffix::User,
            Utc::now(),
            EventPayload::User(UserPayload {
                text: "Sidechain".to_string(),
            }),
            None,
            StreamId::Sidechain {
                agent_id: "test123".to_string(),
            },
        );

        // Another main stream event (should chain from main1)
        let _main2_id = builder.build_and_push(
            &mut events,
            "main-2",
            SemanticSuffix::Message,
            Utc::now(),
            EventPayload::Message(MessagePayload {
                text: "Main 2".to_string(),
            }),
            None,
            StreamId::Main,
        );

        // Verify main stream chain
        assert_eq!(events[0].parent_id, None); // main1
        assert_eq!(events[2].parent_id, Some(main1_id)); // main2

        // Verify sidechain has independent chain
        assert_eq!(events[1].parent_id, None); // side1 (no parent in sidechain)
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
