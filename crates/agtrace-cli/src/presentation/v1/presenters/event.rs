use crate::presentation::v1::view_models::{EventPayloadViewModel, EventViewModel};
use agtrace_types::{AgentEvent, EventPayload};

pub fn present_event(event: &AgentEvent) -> EventViewModel {
    EventViewModel {
        id: event.id.to_string(),
        session_id: event.session_id.to_string(),
        parent_id: event.parent_id.map(|id| id.to_string()),
        timestamp: event.timestamp,
        stream_id: event.stream_id.clone(),
        payload: present_payload(&event.payload),
        metadata: event.metadata.clone(),
    }
}

pub fn present_events(events: &[AgentEvent]) -> Vec<EventViewModel> {
    events.iter().map(present_event).collect()
}

fn present_payload(payload: &EventPayload) -> EventPayloadViewModel {
    match payload {
        EventPayload::User(p) => EventPayloadViewModel::User {
            text: p.text.clone(),
        },
        EventPayload::Reasoning(p) => EventPayloadViewModel::Reasoning {
            text: p.text.clone(),
        },
        EventPayload::ToolCall(p) => {
            let arguments = serde_json::to_value(p)
                .ok()
                .and_then(|v| v.get("arguments").cloned())
                .unwrap_or(serde_json::Value::Null);
            EventPayloadViewModel::ToolCall {
                name: p.name().to_string(),
                arguments,
            }
        }
        EventPayload::ToolResult(p) => EventPayloadViewModel::ToolResult {
            output: p.output.clone(),
            is_error: p.is_error,
        },
        EventPayload::Message(p) => EventPayloadViewModel::Message {
            text: p.text.clone(),
        },
        EventPayload::TokenUsage(p) => EventPayloadViewModel::TokenUsage {
            input: p.input_tokens,
            output: p.output_tokens,
            total: p.total_tokens,
            cache_creation: p
                .details
                .as_ref()
                .and_then(|d| d.cache_creation_input_tokens),
            cache_read: p.details.as_ref().and_then(|d| d.cache_read_input_tokens),
        },
        EventPayload::Notification(p) => EventPayloadViewModel::Notification {
            text: p.text.clone(),
            level: p.level.clone(),
        },
    }
}
