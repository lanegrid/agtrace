use agtrace_types::{v2, AgentEventV1, Channel, EventType, Role, Source, ToolStatus};

/// Convert v2 events to v1 format for legacy analysis/export code
pub fn convert_v2_to_v1(events: &[v2::AgentEvent]) -> Vec<AgentEventV1> {
    let mut v1_events = Vec::new();

    for event in events {
        // Skip TokenUsage events (sidecar, not in v1)
        if matches!(event.payload, v2::EventPayload::TokenUsage(_)) {
            continue;
        }

        // Extract common fields
        let ts = event.timestamp.to_rfc3339();
        let session_id = Some(event.trace_id.to_string());
        let event_id = Some(event.id.to_string());
        let parent_event_id = event.parent_id.map(|id| id.to_string());

        // Convert based on payload type
        match &event.payload {
            v2::EventPayload::User(p) => {
                let mut ev = AgentEventV1::new(
                    Source::new("v2_adapter"),
                    String::new(), // project_hash
                    ts,
                    EventType::UserMessage,
                );
                ev.session_id = session_id;
                ev.event_id = event_id;
                ev.parent_event_id = parent_event_id;
                ev.role = Some(Role::User);
                ev.channel = Some(Channel::Chat);
                ev.text = Some(p.text.clone());
                v1_events.push(ev);
            }
            v2::EventPayload::Reasoning(p) => {
                let mut ev = AgentEventV1::new(
                    Source::new("v2_adapter"),
                    String::new(),
                    ts,
                    EventType::Reasoning,
                );
                ev.session_id = session_id;
                ev.event_id = event_id;
                ev.parent_event_id = parent_event_id;
                ev.role = Some(Role::Assistant);
                ev.text = Some(p.text.clone());
                v1_events.push(ev);
            }
            v2::EventPayload::ToolCall(p) => {
                let mut ev = AgentEventV1::new(
                    Source::new("v2_adapter"),
                    String::new(),
                    ts,
                    EventType::ToolCall,
                );
                ev.session_id = session_id;
                ev.event_id = event_id;
                ev.parent_event_id = parent_event_id;
                ev.role = Some(Role::Assistant);
                ev.tool_name = Some(p.name.clone());
                ev.tool_call_id = Some(event.id.to_string());
                ev.text = Some(p.arguments.to_string());
                v1_events.push(ev);
            }
            v2::EventPayload::ToolResult(p) => {
                let mut ev = AgentEventV1::new(
                    Source::new("v2_adapter"),
                    String::new(),
                    ts,
                    EventType::ToolResult,
                );
                ev.session_id = session_id;
                ev.event_id = event_id;
                ev.parent_event_id = parent_event_id;
                ev.role = Some(Role::Tool);
                ev.tool_call_id = Some(p.tool_call_id.to_string());
                ev.tool_status = Some(if p.is_error {
                    ToolStatus::Error
                } else {
                    ToolStatus::Success
                });
                ev.text = Some(p.output.clone());
                v1_events.push(ev);
            }
            v2::EventPayload::Message(p) => {
                let mut ev = AgentEventV1::new(
                    Source::new("v2_adapter"),
                    String::new(),
                    ts,
                    EventType::AssistantMessage,
                );
                ev.session_id = session_id;
                ev.event_id = event_id;
                ev.parent_event_id = parent_event_id;
                ev.role = Some(Role::Assistant);
                ev.channel = Some(Channel::Chat);
                ev.text = Some(p.text.clone());
                v1_events.push(ev);
            }
            v2::EventPayload::TokenUsage(_) => {
                // Already filtered above, but handle explicitly
            }
        }
    }

    v1_events
}
