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

/// Whether a payload matches a (lowercased) event type pattern.
fn payload_matches(payload: &EventPayload, pattern: &str) -> bool {
    match payload {
        EventPayload::User(_) => pattern == "user",
        EventPayload::Message(_) => pattern == "assistant" || pattern == "message",
        EventPayload::ToolCall(_) | EventPayload::ToolResult(_) => pattern == "tool",
        EventPayload::ToolSubAction(_) => pattern == "tool" || pattern == "subaction",
        EventPayload::Reasoning(_) => pattern == "reasoning",
        EventPayload::TokenUsage(_) => pattern == "token" || pattern == "tokenusage",
        EventPayload::Notification(_) => pattern == "notification" || pattern == "info",
        EventPayload::SlashCommand(_) => pattern == "slashcommand" || pattern == "command",
        EventPayload::QueueOperation(_) => pattern == "queueoperation" || pattern == "queue",
        EventPayload::AgentSpawn(_)
        | EventPayload::AgentLifecycle(_)
        | EventPayload::AgentMessage(_)
        | EventPayload::AgentAttribute(_) => pattern == "agent",
        EventPayload::Compaction(_) => pattern == "compaction",
        EventPayload::TurnEnd(_) => pattern == "turnend" || pattern == "turn",
        EventPayload::ModelChange(_) => pattern == "model" || pattern == "modelchange",
        EventPayload::ContextWindowHint(_) => pattern == "context" || pattern == "contextwindow",
    }
}

/// Filter events by type using inclusion/exclusion patterns.
///
/// Applies `only` filter first (if present), then `hide` filter.
/// Supported patterns: "user", "assistant"/"message", "tool",
/// "reasoning", "token"/"tokenusage", "notification"/"info", "command",
/// "queue", "agent", "compaction", "turn", "model", "context".
pub fn filter_events(events: &[AgentEvent], filters: EventFilters) -> Vec<AgentEvent> {
    let mut filtered = events.to_vec();

    if let Some(only_patterns) = filters.only {
        filtered.retain(|e| {
            only_patterns
                .iter()
                .any(|pattern| payload_matches(&e.payload, &pattern.to_lowercase()))
        });
    }

    if let Some(hide_patterns) = filters.hide {
        filtered.retain(|e| {
            !hide_patterns
                .iter()
                .any(|pattern| payload_matches(&e.payload, &pattern.to_lowercase()))
        });
    }

    filtered
}
