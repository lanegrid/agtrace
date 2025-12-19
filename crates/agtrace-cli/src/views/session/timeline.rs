use crate::display_model::{DisplayOptions, TokenSummaryDisplay};
use crate::views::session::event::format_event_with_start;
use crate::views::session::format_token_summary;
use agtrace_types::{AgentEvent, EventPayload};
use owo_colors::OwoColorize;

// Local summary structures for timeline display
#[derive(Debug, Clone)]
struct TimelineSessionSummary {
    event_counts: TimelineEventCounts,
    token_stats: TimelineTokenStats,
    duration: Option<TimelineDuration>,
}

#[derive(Debug, Clone)]
struct TimelineDuration {
    minutes: i64,
    seconds: i64,
}

#[derive(Debug, Clone)]
struct TimelineEventCounts {
    total: usize,
    user_messages: usize,
    assistant_messages: usize,
    tool_calls: usize,
    reasoning_blocks: usize,
}

#[derive(Debug, Clone)]
struct TimelineTokenStats {
    total: u64,
    input: u64,
    output: u64,
    cached: u64,
    thinking: u64,
}

fn summarize_events(events: &[AgentEvent]) -> TimelineSessionSummary {
    let mut user_count = 0;
    let mut assistant_count = 0;
    let mut tool_call_count = 0;
    let mut reasoning_count = 0;

    let mut total_input = 0i32;
    let mut total_output = 0i32;
    let mut total_cached = 0i32;
    let mut total_thinking = 0i32;

    for event in events {
        match &event.payload {
            EventPayload::User(_) => user_count += 1,
            EventPayload::Message(_) => assistant_count += 1,
            EventPayload::ToolCall(_) => tool_call_count += 1,
            EventPayload::Reasoning(_) => reasoning_count += 1,
            EventPayload::ToolResult(_) => {}
            EventPayload::TokenUsage(token) => {
                total_input += token.input_tokens;
                total_output += token.output_tokens;
                if let Some(details) = &token.details {
                    if let Some(cached) = details.cache_read_input_tokens {
                        total_cached += cached;
                    }
                    if let Some(thinking) = details.reasoning_output_tokens {
                        total_thinking += thinking;
                    }
                }
            }
            EventPayload::Notification(_) => {}
        }
    }

    let duration = if let (Some(first), Some(last)) = (events.first(), events.last()) {
        let first_ts = first.timestamp;
        let last_ts = last.timestamp;
        let duration = last_ts.signed_duration_since(first_ts);
        Some(TimelineDuration {
            minutes: duration.num_minutes(),
            seconds: duration.num_seconds() % 60,
        })
    } else {
        None
    };

    TimelineSessionSummary {
        event_counts: TimelineEventCounts {
            total: events
                .iter()
                .filter(|e| !matches!(e.payload, EventPayload::TokenUsage(_)))
                .count(),
            user_messages: user_count,
            assistant_messages: assistant_count,
            tool_calls: tool_call_count,
            reasoning_blocks: reasoning_count,
        },
        token_stats: TimelineTokenStats {
            total: (total_input + total_output) as u64,
            input: total_input as u64,
            output: total_output as u64,
            cached: total_cached as u64,
            thinking: total_thinking as u64,
        },
        duration,
    }
}

#[allow(dead_code)]
pub fn format_events_timeline(
    events: &[AgentEvent],
    _truncate: bool,
    _enable_color: bool,
) -> Vec<String> {
    let mut lines = Vec::new();

    if events.is_empty() {
        return vec!["No events to display".to_string()];
    }

    let session_start = events.first().map(|e| e.timestamp);

    // Count user messages to determine turn context
    let mut turn_count = 0;
    for event in events.iter() {
        // Skip TokenUsage events in timeline display (shown in summary)
        if matches!(event.payload, EventPayload::TokenUsage(_)) {
            continue;
        }

        // Use the shared formatting function to ensure consistency with watch
        if let Some(line) = format_event_with_start(
            event,
            turn_count, // turn_context (0-indexed)
            None,       // project_root (timeline doesn't have this context)
            session_start,
        ) {
            lines.push(line);
        }

        // Increment turn count after processing user messages
        if matches!(event.payload, EventPayload::User(_)) {
            turn_count += 1;
        }
    }

    // Add session summary
    lines.extend(format_session_summary(events, _enable_color));

    lines
}

pub fn print_events_timeline(events: &[AgentEvent], truncate: bool, enable_color: bool) {
    for line in format_events_timeline(events, truncate, enable_color) {
        println!("{}", line);
    }
}

fn format_session_summary(events: &[AgentEvent], enable_color: bool) -> Vec<String> {
    let mut lines = Vec::new();

    if events.is_empty() {
        return lines;
    }

    let session_summary = summarize_events(events);

    if enable_color {
        lines.push(format!("{}", "---".bright_black()));
        lines.push(format!("{}", "Session Summary:".bright_white().bold()));
    } else {
        lines.push("---".to_string());
        lines.push("Session Summary:".to_string());
    }

    if enable_color {
        lines.push(format!(
            "  {}: {}",
            "Events".cyan(),
            session_summary
                .event_counts
                .total
                .to_string()
                .bright_white()
        ));
        lines.push(format!(
            "    User messages: {}",
            session_summary
                .event_counts
                .user_messages
                .to_string()
                .green()
        ));
        lines.push(format!(
            "    Assistant messages: {}",
            session_summary
                .event_counts
                .assistant_messages
                .to_string()
                .blue()
        ));
        lines.push(format!(
            "    Tool calls: {}",
            session_summary.event_counts.tool_calls.to_string().yellow()
        ));
        lines.push(format!(
            "    Reasoning blocks: {}",
            session_summary
                .event_counts
                .reasoning_blocks
                .to_string()
                .cyan()
        ));
    } else {
        lines.push(format!("  Events: {}", session_summary.event_counts.total));
        lines.push(format!(
            "    User messages: {}",
            session_summary.event_counts.user_messages
        ));
        lines.push(format!(
            "    Assistant messages: {}",
            session_summary.event_counts.assistant_messages
        ));
        lines.push(format!(
            "    Tool calls: {}",
            session_summary.event_counts.tool_calls
        ));
        lines.push(format!(
            "    Reasoning blocks: {}",
            session_summary.event_counts.reasoning_blocks
        ));
    }

    // Use the same token summary format as watch command
    if session_summary.token_stats.total > 0 {
        let token_summary = TokenSummaryDisplay {
            input: session_summary.token_stats.input as i32,
            output: session_summary.token_stats.output as i32,
            cache_creation: 0, // Not tracked separately in timeline
            cache_read: session_summary.token_stats.cached as i32,
            total: session_summary.token_stats.total as i32,
            limit: None, // Timeline doesn't have limit info
            model: None, // Timeline doesn't have model info
            compaction_buffer_pct: None,
        };

        let opts = DisplayOptions {
            enable_color,
            relative_time: false,
            truncate_text: None,
        };

        lines.push(String::new());
        for line in format_token_summary(&token_summary, &opts) {
            lines.push(line);
        }
    }

    if let Some(duration) = session_summary.duration {
        if enable_color {
            lines.push(format!(
                "  {}: {}m {}s",
                "Duration".cyan(),
                duration.minutes,
                duration.seconds
            ));
        } else {
            lines.push(format!(
                "  Duration: {}m {}s",
                duration.minutes, duration.seconds
            ));
        }
    }

    lines
}
