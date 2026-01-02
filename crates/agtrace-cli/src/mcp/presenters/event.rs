use agtrace_sdk::types::{AgentEvent, EventPayload};
use chrono::{DateTime, Utc};

use crate::mcp::models::common::EventType;
use crate::mcp::models::response::{
    EventDetailsViewModel, EventPreviewViewModel, PreviewContent, SearchEventPreviewsViewModel,
};

pub fn present_event_preview(
    session_id: String,
    event_index: usize,
    event: &AgentEvent,
) -> EventPreviewViewModel {
    let event_type = event_type_from_payload(&event.payload);
    EventPreviewViewModel {
        session_id,
        event_index,
        timestamp: event.timestamp,
        event_type,
        preview: PreviewContent::from_payload(&event.payload),
    }
}

pub fn present_search_event_previews(
    matches: Vec<EventPreviewViewModel>,
) -> SearchEventPreviewsViewModel {
    SearchEventPreviewsViewModel { matches }
}

pub fn present_event_details(
    session_id: String,
    event_index: usize,
    timestamp: DateTime<Utc>,
    payload: EventPayload,
) -> EventDetailsViewModel {
    let event_type = event_type_from_payload(&payload);
    EventDetailsViewModel {
        session_id,
        event_index,
        timestamp,
        event_type,
        payload,
    }
}

fn event_type_from_payload(payload: &EventPayload) -> EventType {
    match payload {
        EventPayload::ToolCall(_) => EventType::ToolCall,
        EventPayload::ToolResult(_) => EventType::ToolResult,
        EventPayload::Message(_) => EventType::Message,
        EventPayload::User(_) => EventType::User,
        EventPayload::Reasoning(_) => EventType::Reasoning,
        EventPayload::TokenUsage(_) => EventType::TokenUsage,
        EventPayload::Notification(_) => EventType::Notification,
    }
}
