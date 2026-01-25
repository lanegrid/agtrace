use agtrace_types::{AgentEvent, EventPayload};

/// Filters for including/excluding event types.
///
/// Supports both allowlist (`only`) and blocklist (`hide`) patterns
/// for event type filtering.
pub struct EventFilters {
    /// Event types to exclude (blocklist).
    pub hide: Option<Vec<String>>,
    /// Event types to include exclusively (allowlist).
    pub only: Option<Vec<String>>,
}

/// Filter events by type using inclusion/exclusion patterns.
///
/// Applies `only` filter first (if present), then `hide` filter.
/// Supported patterns: "user", "assistant"/"message", "tool",
/// "reasoning", "token"/"tokenusage", "notification"/"info".
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
                    EventPayload::SlashCommand(_) => {
                        pattern_lower == "slashcommand" || pattern_lower == "command"
                    }
                    EventPayload::QueueOperation(_) => {
                        pattern_lower == "queueoperation" || pattern_lower == "queue"
                    }
                    EventPayload::Summary(_) => pattern_lower == "summary",
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
                    EventPayload::SlashCommand(_) => {
                        pattern_lower == "slashcommand" || pattern_lower == "command"
                    }
                    EventPayload::QueueOperation(_) => {
                        pattern_lower == "queueoperation" || pattern_lower == "queue"
                    }
                    EventPayload::Summary(_) => pattern_lower == "summary",
                }
            })
        });
    }

    filtered
}
