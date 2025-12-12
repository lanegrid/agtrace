use agtrace_engine::summarize_session;
use agtrace_types::{AgentEventV1, EventType};
use chrono::DateTime;
use owo_colors::OwoColorize;

pub fn print_events_timeline(events: &[AgentEventV1], truncate: bool, enable_color: bool) {
    if events.is_empty() {
        let msg = "No events to display";
        if enable_color {
            println!("{}", msg.bright_black());
        } else {
            println!("{}", msg);
        }
        return;
    }

    let session_start = events
        .first()
        .and_then(|e| DateTime::parse_from_rfc3339(&e.ts).ok());

    for event in events {
        // Calculate relative time from session start
        let time_display = if let (Some(start), Ok(current)) =
            (session_start, DateTime::parse_from_rfc3339(&event.ts))
        {
            let duration = current.signed_duration_since(start);
            let seconds = duration.num_seconds();
            if seconds < 60 {
                format!("[+{}s    ]", seconds)
            } else {
                let minutes = seconds / 60;
                let secs = seconds % 60;
                format!("[+{}m {:02}s]", minutes, secs)
            }
        } else {
            format!("[{}]", &event.ts[11..19]) // fallback to HH:MM:SS
        };

        // Event type string with optional color
        let event_type_name = match event.event_type {
            EventType::UserMessage => "UserMessage",
            EventType::AssistantMessage => "AssistantMessage",
            EventType::Reasoning => "Reasoning",
            EventType::ToolCall => "ToolCall",
            EventType::ToolResult => "ToolResult",
            EventType::SystemMessage => "SystemMessage",
            EventType::FileSnapshot => "FileSnapshot",
            EventType::SessionSummary => "SessionSummary",
            EventType::Meta => "Meta",
            EventType::Log => "Log",
        };

        // Add status indicator for ToolResult events
        let status_indicator = if matches!(event.event_type, EventType::ToolResult) {
            match event.tool_status {
                Some(agtrace_types::ToolStatus::Success) => {
                    if enable_color {
                        format!("{} ", "✓".green())
                    } else {
                        "✓ ".to_string()
                    }
                }
                Some(agtrace_types::ToolStatus::Error) => {
                    if enable_color {
                        format!("{} ", "✗".red())
                    } else {
                        "✗ ".to_string()
                    }
                }
                _ => String::new(),
            }
        } else {
            String::new()
        };

        let event_type_str = if enable_color {
            match event.event_type {
                EventType::UserMessage => format!("{}", event_type_name.green()),
                EventType::AssistantMessage => format!("{}", event_type_name.blue()),
                EventType::Reasoning => format!("{}", event_type_name.cyan()),
                EventType::ToolCall => format!("{}", event_type_name.yellow()),
                EventType::ToolResult => {
                    // Color ToolResult based on status
                    match event.tool_status {
                        Some(agtrace_types::ToolStatus::Success) => {
                            format!("{}", event_type_name.green())
                        }
                        Some(agtrace_types::ToolStatus::Error) => {
                            format!("{}", event_type_name.red())
                        }
                        _ => format!("{}", event_type_name.magenta()),
                    }
                }
                EventType::SystemMessage => format!("{}", event_type_name.white()),
                EventType::FileSnapshot => format!("{}", event_type_name.bright_black()),
                EventType::SessionSummary => format!("{}", event_type_name.bright_blue()),
                EventType::Meta => format!("{}", event_type_name.bright_black()),
                EventType::Log => format!("{}", event_type_name.bright_black()),
            }
        } else {
            event_type_name.to_string()
        };

        let role_str = event
            .role
            .map(|r| {
                let s = format!("(role={:?})", r);
                if enable_color {
                    format!("{}", s.bright_black())
                } else {
                    s
                }
            })
            .unwrap_or_else(|| "".to_string());

        let time_colored = if enable_color {
            format!("{}", time_display.bright_black())
        } else {
            time_display
        };

        println!(
            "{} {}{:<20} {}",
            time_colored, status_indicator, event_type_str, role_str
        );

        if let Some(text) = &event.text {
            let preview = if truncate && text.chars().count() > 100 {
                // Only truncate if --short flag is used AND text is long
                let truncated: String = text.chars().take(97).collect();
                format!("{}...", truncated)
            } else {
                // Default: show full text
                text.clone()
            };

            let text_output = if enable_color {
                format!("{}", preview.white())
            } else {
                preview
            };
            println!("  {}", text_output);
        }

        if let Some(tool_name) = &event.tool_name {
            if enable_color {
                print!("  tool: {}", tool_name.yellow());
            } else {
                print!("  tool: {}", tool_name);
            }

            if let Some(file_path) = &event.file_path {
                if enable_color {
                    print!(" ({})", file_path.bright_blue());
                } else {
                    print!(" ({})", file_path);
                }
            }

            if let Some(file_op) = &event.file_op {
                if enable_color {
                    print!(" [{}]", file_op.bright_cyan());
                } else {
                    print!(" [{}]", file_op);
                }
            }

            // Show status for ToolResult events
            if matches!(event.event_type, EventType::ToolResult) {
                if let Some(status) = &event.tool_status {
                    let status_str = format!("{:?}", status).to_lowercase();
                    if enable_color {
                        let status_colored = match status {
                            agtrace_types::ToolStatus::Success => {
                                format!("{}", status_str.green().bold())
                            }
                            agtrace_types::ToolStatus::Error => {
                                format!("{}", status_str.red().bold())
                            }
                            _ => format!("{}", status_str.yellow()),
                        };
                        print!(" {}", status_colored);
                    } else {
                        print!(" {}", status_str);
                    }
                }
            }

            if let Some(exit_code) = event.tool_exit_code {
                let exit_str = format!("exit={}", exit_code);
                if enable_color {
                    let exit_colored = if exit_code == 0 {
                        format!("{}", exit_str.green())
                    } else {
                        format!("{}", exit_str.red())
                    };
                    print!(" {}", exit_colored);
                } else {
                    print!(" {}", exit_str);
                }
            }
            println!();
        }

        // Display token information for assistant messages
        if matches!(event.event_type, EventType::AssistantMessage) {
            let mut token_parts = Vec::new();
            if let Some(input) = event.tokens_input {
                token_parts.push(format!("in:{}", input));
            }
            if let Some(output) = event.tokens_output {
                token_parts.push(format!("out:{}", output));
            }
            if let Some(cached) = event.tokens_cached {
                if cached > 0 {
                    token_parts.push(format!("cached:{}", cached));
                }
            }
            if let Some(thinking) = event.tokens_thinking {
                if thinking > 0 {
                    token_parts.push(format!("thinking:{}", thinking));
                }
            }
            if let Some(tool) = event.tokens_tool {
                if tool > 0 {
                    token_parts.push(format!("tool:{}", tool));
                }
            }
            if !token_parts.is_empty() {
                let tokens_str = token_parts.join(", ");
                if enable_color {
                    println!("  tokens: {}", tokens_str.bright_black());
                } else {
                    println!("  tokens: {}", tokens_str);
                }
            }
        }

        println!();
    }

    // Print session summary
    print_session_summary(events, enable_color);
}

fn print_session_summary(events: &[AgentEventV1], enable_color: bool) {
    if events.is_empty() {
        return;
    }

    let session_summary = summarize_session(events);

    if enable_color {
        println!("{}", "---".bright_black());
        println!(
            "{}",
            "Session Summary:".bright_white().bold()
        );
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

    if !session_summary.file_operations.is_empty() {
        if enable_color {
            println!("  {}:", "File operations".cyan());
        } else {
            println!("  File operations:");
        }
        for (op, count) in session_summary.file_operations.iter() {
            if enable_color {
                println!(
                    "    {}: {}",
                    op,
                    count.to_string().bright_white()
                );
            } else {
                println!("    {}: {}", op, count);
            }
        }
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
