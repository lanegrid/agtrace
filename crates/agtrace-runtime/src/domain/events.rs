use agtrace_types::{AgentEvent, EventPayload};

pub struct EventFilters {
    pub hide: Option<Vec<String>>,
    pub only: Option<Vec<String>>,
}

pub fn filter_events(events: &[AgentEvent], filters: EventFilters) -> Vec<AgentEvent> {
    let mut filtered = events.to_vec();

    if let Some(only_patterns) = filters.only {
        filtered.retain(|e| {
            only_patterns.iter().any(|pattern| {
                let pattern_lower = pattern.to_lowercase();
                match &e.payload {
                    EventPayload::User(_) => pattern_lower == "user",
                    EventPayload::Message(_) => {
                        pattern_lower == "assistant" || pattern_lower == "message"
                    }
                    EventPayload::ToolCall(_) | EventPayload::ToolResult(_) => {
                        pattern_lower == "tool"
                    }
                    EventPayload::Reasoning(_) => pattern_lower == "reasoning",
                    EventPayload::TokenUsage(_) => {
                        pattern_lower == "token" || pattern_lower == "tokenusage"
                    }
                    EventPayload::Notification(_) => {
                        pattern_lower == "notification" || pattern_lower == "info"
                    }
                }
            })
        });
    }

    if let Some(hide_patterns) = filters.hide {
        filtered.retain(|e| {
            !hide_patterns.iter().any(|pattern| {
                let pattern_lower = pattern.to_lowercase();
                match &e.payload {
                    EventPayload::User(_) => pattern_lower == "user",
                    EventPayload::Message(_) => {
                        pattern_lower == "assistant" || pattern_lower == "message"
                    }
                    EventPayload::ToolCall(_) | EventPayload::ToolResult(_) => {
                        pattern_lower == "tool"
                    }
                    EventPayload::Reasoning(_) => pattern_lower == "reasoning",
                    EventPayload::TokenUsage(_) => {
                        pattern_lower == "token" || pattern_lower == "tokenusage"
                    }
                    EventPayload::Notification(_) => {
                        pattern_lower == "notification" || pattern_lower == "info"
                    }
                }
            })
        });
    }

    filtered
}
