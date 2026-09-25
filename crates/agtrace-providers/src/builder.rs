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
    SlashCommand,
    QueueOperation,
    AgentSpawn,
    AgentLifecycle,
    AgentMessage,
    Compaction,
    TurnEnd,
    ModelChange,
    ContextWindowHint,
    AgentAttribute,
    ToolSubAction,
    Plan,
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
            Self::SlashCommand => "slashcmd",
            Self::QueueOperation => "queue",
            Self::AgentSpawn => "spawn",
            Self::AgentLifecycle => "lifecycle",
            Self::AgentMessage => "agentmsg",
            Self::Compaction => "compaction",
            Self::TurnEnd => "turnend",
            Self::ModelChange => "model",
            Self::ContextWindowHint => "ctxhint",
            Self::AgentAttribute => "attr",
            Self::ToolSubAction => "subaction",
            Self::Plan => "plan",
        }
    }
}

/// EventBuilder helps convert provider raw data to events.
///
/// Maintains per-agent parent chains (tip per [`AgentId`]), the provider tool call
/// id -> event UUID map (persists across lines), and the [`EventOrigin`] of the
/// line currently being decoded.
pub struct EventBuilder {
    /// Current session ID
    pub session_id: Uuid,

    /// Most recent event ID per agent (independent parent chains per agent).
    agent_tips: HashMap<AgentId, Uuid>,

    /// Provider tool call ID -> UUID mapping
    tool_map: HashMap<String, Uuid>,

    /// Origin of the line currently being decoded; `sub` counts events of that line.
    origin: EventOrigin,
}

impl EventBuilder {
    pub fn new(session_id: Uuid) -> Self {
        Self {
            session_id,
            agent_tips: HashMap::new(),
            tool_map: HashMap::new(),
            origin: EventOrigin::default(),
        }
    }

    /// Start a new source line: subsequent events get this line's origin.
    pub fn begin_line(&mut self, line: u64, byte_offset: u64) {
        self.origin = EventOrigin::new(line, byte_offset, 0);
    }

    /// Create and push event with deterministic UUID generation
    /// Uses UUID v5 with session_id as namespace and "base_id:suffix" as name
    /// Returns the generated event ID
    pub fn build_and_push(
        &mut self,
        events: &mut Vec<AgentEvent>,
        base_id: &str,
        suffix: SemanticSuffix,
        timestamp: DateTime<Utc>,
        payload: EventPayload,
        agent: &AgentId,
    ) -> Uuid {
        // Generate deterministic UUID: session_id namespace + "base_id:suffix" name
        let name = format!("{}:{}", base_id, suffix.as_str());
        let id = Uuid::new_v5(&self.session_id, name.as_bytes());

        // Get parent_id from agent-specific tip
        let parent_id = self.agent_tips.get(agent).copied();

        events.push(AgentEvent {
            id,
            session_id: self.session_id,
            agent: agent.clone(),
            parent_id,
            timestamp,
            origin: self.origin,
            payload,
        });
        self.origin.sub = self.origin.sub.saturating_add(1);

        // Update agent tip
        match self.agent_tips.get_mut(agent) {
            Some(tip) => *tip = id,
            None => {
                self.agent_tips.insert(agent.clone(), id);
            }
        }
        id
    }

    /// Register a tool call in the map (provider ID -> UUID)
    pub fn register_tool_call(&mut self, provider_id: String, uuid: Uuid) {
        self.tool_map.insert(provider_id, uuid);
    }

    /// Get UUID for a provider tool call ID
    pub fn get_tool_call_uuid(&self, provider_id: &str) -> Option<Uuid> {
        self.tool_map.get(provider_id).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claude::ClaudeToolMapper;
    use crate::traits::ToolMapper;

    fn main_agent() -> AgentId {
        AgentId::claude_session("s1")
    }

    #[test]
    fn test_event_builder_chain() {
        let session_id = Uuid::new_v4();
        let mut builder = EventBuilder::new(session_id);
        let mut events = Vec::new();
        let agent = main_agent();

        // First event has no parent
        builder.begin_line(0, 0);
        let event1_id = builder.build_and_push(
            &mut events,
            "test-id-1",
            SemanticSuffix::User,
            Utc::now(),
            EventPayload::User(UserPayload {
                text: "Hello".to_string(),
            }),
            &agent,
        );
        assert_eq!(events[0].parent_id, None);
        assert_eq!(events[0].session_id, session_id);
        assert_eq!(events[0].agent, agent);

        // Second event has first as parent
        builder.begin_line(1, 10);
        let event2_id = builder.build_and_push(
            &mut events,
            "test-id-2",
            SemanticSuffix::Message,
            Utc::now(),
            EventPayload::Message(MessagePayload::new("Hi")),
            &agent,
        );
        assert_eq!(events[1].parent_id, Some(event1_id));

        // Third event has second as parent (same line => sub increments)
        let mapper = ClaudeToolMapper;
        builder.build_and_push(
            &mut events,
            "test-id-3",
            SemanticSuffix::ToolCall,
            Utc::now(),
            EventPayload::ToolCall(mapper.normalize_call(
                "Bash",
                serde_json::json!({"command": "ls"}),
                Some("call_123".to_string()),
            )),
            &agent,
        );
        assert_eq!(events[2].parent_id, Some(event2_id));
        assert_eq!(events[1].origin, EventOrigin::new(1, 10, 0));
        assert_eq!(events[2].origin, EventOrigin::new(1, 10, 1));
    }

    #[test]
    fn test_multi_agent_chains() {
        let session_id = Uuid::new_v4();
        let mut builder = EventBuilder::new(session_id);
        let mut events = Vec::new();
        let main = main_agent();
        let sub = AgentId::claude_subagent("s1", "test123");

        let main1_id = builder.build_and_push(
            &mut events,
            "main-1",
            SemanticSuffix::User,
            Utc::now(),
            EventPayload::User(UserPayload {
                text: "Main".to_string(),
            }),
            &main,
        );

        let _side1_id = builder.build_and_push(
            &mut events,
            "side-1",
            SemanticSuffix::User,
            Utc::now(),
            EventPayload::User(UserPayload {
                text: "Sidechain".to_string(),
            }),
            &sub,
        );

        let _main2_id = builder.build_and_push(
            &mut events,
            "main-2",
            SemanticSuffix::Message,
            Utc::now(),
            EventPayload::Message(MessagePayload::new("Main 2")),
            &main,
        );

        assert_eq!(events[0].parent_id, None); // main1
        assert_eq!(events[2].parent_id, Some(main1_id)); // main2
        assert_eq!(events[1].parent_id, None); // subagent has an independent chain
    }

    #[test]
    fn test_tool_map() {
        let mut builder = EventBuilder::new(Uuid::new_v4());
        let tool_uuid = Uuid::new_v4();

        builder.register_tool_call("tool-123".to_string(), tool_uuid);

        assert_eq!(builder.get_tool_call_uuid("tool-123"), Some(tool_uuid));
        assert_eq!(builder.get_tool_call_uuid("nonexistent"), None);
    }
}
