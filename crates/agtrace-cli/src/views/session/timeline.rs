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

pub fn print_events_timeline(events: &[AgentEvent], truncate: bool, enable_color: bool) {
    if events.is_empty() {
        let msg = "No events to display";
        if enable_color {
            println!("{}", msg.bright_black());
        } else {
            println!("{}", msg);
        }
        return;
    }

    let session_start = events.first().map(|e| e.timestamp);

    for event in events {
        // Skip TokenUsage events in timeline display (shown in summary)
        if matches!(event.payload, EventPayload::TokenUsage(_)) {
            continue;
        }

        // Calculate relative time from session start
        let time_display = if let Some(start) = session_start {
            let duration = event.timestamp.signed_duration_since(start);
            let seconds = duration.num_seconds();
            if seconds < 60 {
                format!("[+{}s    ]", seconds)
            } else {
                let minutes = seconds / 60;
                let secs = seconds % 60;
                format!("[+{}m {:02}s]", minutes, secs)
            }
        } else {
            let ts_str = event.timestamp.to_rfc3339();
            format!("[{}]", &ts_str[11..19]) // fallback to HH:MM:SS
        };

        // Event type string with optional color
        let (event_type_name, text_opt, tool_name_opt, is_error) = match &event.payload {
            EventPayload::User(p) => {
                // Skip empty user messages
                if p.text.trim().is_empty() {
                    continue;
                }
                ("UserMessage", Some(&p.text), None, false)
            }
            EventPayload::Message(p) => {
                // Skip empty assistant messages (common in Gemini when only tool calls are present)
                if p.text.trim().is_empty() {
                    continue;
                }
                ("AssistantMessage", Some(&p.text), None, false)
            }
            EventPayload::Reasoning(p) => {
                // Skip empty reasoning blocks
                if p.text.trim().is_empty() {
                    continue;
                }
                ("Reasoning", Some(&p.text), None, false)
            }
            EventPayload::ToolCall(p) => ("ToolCall", Some(&p.name), Some(&p.name), false),
            EventPayload::ToolResult(p) => ("ToolResult", Some(&p.output), None, p.is_error),
            EventPayload::TokenUsage(_) => continue, // Skip (already filtered above)
            EventPayload::Notification(p) => ("Notification", Some(&p.text), None, false),
        };

        // Add status indicator for ToolResult events
        let status_indicator = if matches!(event.payload, EventPayload::ToolResult(_)) {
            if is_error {
                if enable_color {
                    format!("{} ", "✗".red())
                } else {
                    "✗ ".to_string()
                }
            } else if enable_color {
                format!("{} ", "✓".green())
            } else {
                "✓ ".to_string()
            }
        } else {
            String::new()
        };

        let event_type_str = if enable_color {
            match &event.payload {
                EventPayload::User(_) => format!("{}", event_type_name.green()),
                EventPayload::Message(_) => format!("{}", event_type_name.blue()),
                EventPayload::Reasoning(_) => format!("{}", event_type_name.cyan()),
                EventPayload::ToolCall(_) => format!("{}", event_type_name.yellow()),
                EventPayload::ToolResult(_) => {
                    if is_error {
                        format!("{}", event_type_name.red())
                    } else {
                        format!("{}", event_type_name.green())
                    }
                }
                EventPayload::TokenUsage(_) => continue,
                EventPayload::Notification(_) => format!("{}", event_type_name.cyan()),
            }
        } else {
            event_type_name.to_string()
        };

        let time_colored = if enable_color {
            format!("{}", time_display.bright_black())
        } else {
            time_display
        };

        println!(
            "{} {}{:<20}",
            time_colored, status_indicator, event_type_str
        );

        if let Some(text) = text_opt {
            let preview = if truncate && text.chars().count() > 100 {
                let truncated: String = text.chars().take(97).collect();
                format!("{}...", truncated)
            } else {
                text.clone()
            };

            let text_output = if enable_color {
                format!("{}", preview.white())
            } else {
                preview
            };
            println!("  {}", text_output);
        }

        if let Some(tool_name) = tool_name_opt {
            if enable_color {
                println!("  tool: {}", tool_name.yellow());
            } else {
                println!("  tool: {}", tool_name);
            }
        }

        println!();
    }

    // Print session summary
    print_session_summary(events, enable_color);
}

fn print_session_summary(events: &[AgentEvent], enable_color: bool) {
    if events.is_empty() {
        return;
    }

    let session_summary = summarize_events(events);

    if enable_color {
        println!("{}", "---".bright_black());
        println!("{}", "Session Summary:".bright_white().bold());
    } else {
        println!("---");
        println!("Session Summary:");
    }

    if enable_color {
        println!(
            "  {}: {}",
            "Events".cyan(),
            session_summary
                .event_counts
                .total
                .to_string()
                .bright_white()
        );
        println!(
            "    User messages: {}",
            session_summary
                .event_counts
                .user_messages
                .to_string()
                .green()
        );
        println!(
            "    Assistant messages: {}",
            session_summary
                .event_counts
                .assistant_messages
                .to_string()
                .blue()
        );
        println!(
            "    Tool calls: {}",
            session_summary.event_counts.tool_calls.to_string().yellow()
        );
        println!(
            "    Reasoning blocks: {}",
            session_summary
                .event_counts
                .reasoning_blocks
                .to_string()
                .cyan()
        );
    } else {
        println!("  Events: {}", session_summary.event_counts.total);
        println!(
            "    User messages: {}",
            session_summary.event_counts.user_messages
        );
        println!(
            "    Assistant messages: {}",
            session_summary.event_counts.assistant_messages
        );
        println!(
            "    Tool calls: {}",
            session_summary.event_counts.tool_calls
        );
        println!(
            "    Reasoning blocks: {}",
            session_summary.event_counts.reasoning_blocks
        );
    }

    if session_summary.token_stats.total > 0 {
        if enable_color {
            println!(
                "  {}: {}",
                "Tokens".cyan(),
                session_summary.token_stats.total.to_string().bright_white()
            );
            println!(
                "    Input: {}",
                session_summary.token_stats.input.to_string().bright_white()
            );
            println!(
                "    Output: {}",
                session_summary
                    .token_stats
                    .output
                    .to_string()
                    .bright_white()
            );
            if session_summary.token_stats.cached > 0 {
                println!(
                    "    Cached: {}",
                    session_summary
                        .token_stats
                        .cached
                        .to_string()
                        .bright_yellow()
                );
            }
            if session_summary.token_stats.thinking > 0 {
                println!(
                    "    Thinking: {}",
                    session_summary
                        .token_stats
                        .thinking
                        .to_string()
                        .bright_cyan()
                );
            }
        } else {
            println!("  Tokens: {}", session_summary.token_stats.total);
            println!("    Input: {}", session_summary.token_stats.input);
            println!("    Output: {}", session_summary.token_stats.output);
            if session_summary.token_stats.cached > 0 {
                println!("    Cached: {}", session_summary.token_stats.cached);
            }
            if session_summary.token_stats.thinking > 0 {
                println!("    Thinking: {}", session_summary.token_stats.thinking);
            }
        }
    }

    if let Some(duration) = session_summary.duration {
        if enable_color {
            println!(
                "  {}: {}m {}s",
                "Duration".cyan(),
                duration.minutes,
                duration.seconds
            );
        } else {
            println!("  Duration: {}m {}s", duration.minutes, duration.seconds);
        }
    }
}
