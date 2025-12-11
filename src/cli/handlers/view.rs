#![allow(clippy::format_in_format_args)] // Intentional for colored terminal output

use crate::core::{aggregate_activities, Activity};
use crate::db::Database;
use crate::model::{AgentEventV1, EventType, Role};
use crate::providers::{ClaudeProvider, CodexProvider, GeminiProvider, ImportContext, LogProvider};
use anyhow::{Context, Result};
use chrono::DateTime;
use is_terminal::IsTerminal;
use owo_colors::OwoColorize;
use std::fs;
use std::io;
use std::path::Path;

#[allow(clippy::too_many_arguments)]
pub fn handle(
    db: &Database,
    session_id: String,
    raw: bool,
    json: bool,
    _timeline: bool,
    hide: Option<Vec<String>>,
    only: Option<Vec<String>>,
    _full: bool, // Kept for backwards compatibility, but now default
    short: bool,
    style: String,
) -> Result<()> {
    // Detect if output is being piped (not a terminal)
    let is_tty = io::stdout().is_terminal();
    let enable_color = is_tty;
    // Try to resolve session ID (supports prefix matching)
    let resolved_id = match db.find_session_by_prefix(&session_id)? {
        Some(full_id) => full_id,
        None => {
            // If prefix matching fails, try exact match
            let files = db.get_session_files(&session_id)?;
            if files.is_empty() {
                anyhow::bail!("Session not found: {}", session_id);
            }
            session_id.clone()
        }
    };

    let log_files = db.get_session_files(&resolved_id)?;

    if log_files.is_empty() {
        anyhow::bail!("Session not found: {}", session_id);
    }

    // Filter out sidechain files (e.g., Claude's agent-*.jsonl)
    let main_files: Vec<_> = log_files
        .into_iter()
        .filter(|f| f.role != "sidechain")
        .collect();

    if main_files.is_empty() {
        anyhow::bail!("No main log files found for session: {}", session_id);
    }

    if raw {
        for log_file in &main_files {
            let content = fs::read_to_string(&log_file.path)
                .with_context(|| format!("Failed to read file: {}", log_file.path))?;
            println!("{}", content);
        }
        return Ok(());
    }

    let mut all_events = Vec::new();

    for log_file in &main_files {
        let path = Path::new(&log_file.path);
        let provider: Box<dyn LogProvider> = if log_file.path.contains(".claude/") {
            Box::new(ClaudeProvider::new())
        } else if log_file.path.contains(".codex/") {
            Box::new(CodexProvider::new())
        } else if log_file.path.contains(".gemini/") {
            Box::new(GeminiProvider::new())
        } else {
            eprintln!("Warning: Unknown provider for file: {}", log_file.path);
            continue;
        };

        let context = ImportContext {
            project_root_override: None,
            session_id_prefix: None,
            all_projects: false,
        };

        match provider.normalize_file(path, &context) {
            Ok(mut events) => {
                all_events.append(&mut events);
            }
            Err(e) => {
                eprintln!("Warning: Failed to normalize {}: {}", log_file.path, e);
            }
        }
    }

    all_events.sort_by(|a, b| a.ts.cmp(&b.ts));

    // Filter events based on --hide and --only options
    let filtered_events = filter_events(&all_events, hide.as_ref(), only.as_ref());

    if json {
        println!("{}", serde_json::to_string_pretty(&filtered_events)?);
    } else if style == "compact" {
        let activities = aggregate_activities(&filtered_events);
        print_activities_compact(&activities, enable_color);
    } else {
        // Default is full display, --short enables truncation
        let truncate = short;
        print_events_timeline(&filtered_events, truncate, enable_color);
    }

    Ok(())
}

/// Filter events based on hide/only patterns
fn filter_events(
    events: &[AgentEventV1],
    hide: Option<&Vec<String>>,
    only: Option<&Vec<String>>,
) -> Vec<AgentEventV1> {
    let mut filtered = events.to_vec();

    // Apply --only filter (whitelist)
    if let Some(only_patterns) = only {
        filtered.retain(|e| {
            let event_type = format!("{:?}", e.event_type).to_lowercase();
            only_patterns.iter().any(|pattern| {
                let pattern_lower = pattern.to_lowercase();
                event_type.contains(&pattern_lower)
                    || pattern_lower == "user" && matches!(e.event_type, EventType::UserMessage)
                    || pattern_lower == "assistant"
                        && matches!(e.event_type, EventType::AssistantMessage)
                    || pattern_lower == "tool"
                        && (matches!(e.event_type, EventType::ToolCall)
                            || matches!(e.event_type, EventType::ToolResult))
                    || pattern_lower == "reasoning" && matches!(e.event_type, EventType::Reasoning)
            })
        });
    }

    // Apply --hide filter (blacklist)
    if let Some(hide_patterns) = hide {
        filtered.retain(|e| {
            let event_type = format!("{:?}", e.event_type).to_lowercase();
            !hide_patterns.iter().any(|pattern| {
                let pattern_lower = pattern.to_lowercase();
                event_type.contains(&pattern_lower)
                    || pattern_lower == "user" && matches!(e.event_type, EventType::UserMessage)
                    || pattern_lower == "assistant"
                        && matches!(e.event_type, EventType::AssistantMessage)
                    || pattern_lower == "tool"
                        && (matches!(e.event_type, EventType::ToolCall)
                            || matches!(e.event_type, EventType::ToolResult))
                    || pattern_lower == "reasoning" && matches!(e.event_type, EventType::Reasoning)
            })
        });
    }

    filtered
}

fn print_events_timeline(events: &[AgentEventV1], truncate: bool, enable_color: bool) {
    if events.is_empty() {
        let msg = "No events to display";
        if enable_color {
            println!("{}", format!("{}", msg.bright_black()));
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
                Some(crate::model::ToolStatus::Success) => {
                    if enable_color {
                        format!("{} ", "✓".green())
                    } else {
                        "✓ ".to_string()
                    }
                }
                Some(crate::model::ToolStatus::Error) => {
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
                        Some(crate::model::ToolStatus::Success) => {
                            format!("{}", event_type_name.green())
                        }
                        Some(crate::model::ToolStatus::Error) => {
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
                print!("  tool: {}", format!("{}", tool_name.yellow()));
            } else {
                print!("  tool: {}", tool_name);
            }

            if let Some(file_path) = &event.file_path {
                if enable_color {
                    print!(" ({})", format!("{}", file_path.bright_blue()));
                } else {
                    print!(" ({})", file_path);
                }
            }

            if let Some(file_op) = &event.file_op {
                if enable_color {
                    print!(" [{}]", format!("{}", file_op.bright_cyan()));
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
                            crate::model::ToolStatus::Success => {
                                format!("{}", status_str.green().bold())
                            }
                            crate::model::ToolStatus::Error => {
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
                    println!("  tokens: {}", format!("{}", tokens_str.bright_black()));
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

fn print_activities_compact(activities: &[Activity], enable_color: bool) {
    if activities.is_empty() {
        let msg = "No events to display";
        if enable_color {
            println!("{}", format!("{}", msg.bright_black()));
        } else {
            println!("{}", msg);
        }
        return;
    }

    let session_start = activities
        .first()
        .and_then(|a| DateTime::parse_from_rfc3339(a.timestamp()).ok());

    for activity in activities {
        match activity {
            Activity::Message {
                role,
                text,
                timestamp,
                ..
            } => {
                let time_display = if let (Some(start), Ok(current)) =
                    (session_start, DateTime::parse_from_rfc3339(timestamp))
                {
                    let duration = current.signed_duration_since(start);
                    let seconds = duration.num_seconds();
                    if seconds < 60 {
                        format!("[+{:02}:{:02}]", 0, seconds)
                    } else {
                        let minutes = seconds / 60;
                        let secs = seconds % 60;
                        format!("[+{:02}:{:02}]", minutes, secs)
                    }
                } else {
                    format!("[{}]", &timestamp[11..19])
                };

                let time_colored = if enable_color {
                    format!("{}", time_display.bright_black())
                } else {
                    time_display.clone()
                };

                let dur_placeholder = "   -   ";
                let dur_colored = if enable_color {
                    format!("{}", dur_placeholder.bright_black())
                } else {
                    dur_placeholder.to_string()
                };

                let role_str = if matches!(role, Role::User) {
                    "User"
                } else {
                    "Asst"
                };

                if enable_color {
                    let colored_text = if matches!(role, Role::User) {
                        format!("{}", text.green())
                    } else {
                        format!("{}", text.blue())
                    };
                    println!(
                        "{} {} {}: \"{}\"",
                        time_colored, dur_colored, role_str, colored_text
                    );
                } else {
                    println!(
                        "{} {} {}: \"{}\"",
                        time_display, dur_placeholder, role_str, text
                    );
                }
            }

            Activity::Execution {
                timestamp,
                duration_ms,
                tools,
                ..
            } => {
                let time_display = if let (Some(start), Ok(current)) =
                    (session_start, DateTime::parse_from_rfc3339(timestamp))
                {
                    let duration = current.signed_duration_since(start);
                    let seconds = duration.num_seconds();
                    if seconds < 60 {
                        format!("[+{:02}:{:02}]", 0, seconds)
                    } else {
                        let minutes = seconds / 60;
                        let secs = seconds % 60;
                        format!("[+{:02}:{:02}]", minutes, secs)
                    }
                } else {
                    "[+00:00]".to_string()
                };

                let time_colored = if enable_color {
                    format!("{}", time_display.bright_black())
                } else {
                    time_display.clone()
                };

                let dur_str = if *duration_ms >= 1000 {
                    format!("{:2}s    ", duration_ms / 1000)
                } else if *duration_ms > 0 {
                    format!("{:3}ms  ", duration_ms)
                } else {
                    "       ".to_string()
                };

                let dur_colored = if enable_color {
                    if *duration_ms > 30000 {
                        format!("{}", dur_str.red())
                    } else if *duration_ms > 10000 {
                        format!("{}", dur_str.yellow())
                    } else {
                        format!("{}", dur_str.bright_black())
                    }
                } else {
                    dur_str.clone()
                };

                let flow = format_tool_flow(tools, enable_color);

                if enable_color {
                    println!("{} {} {}", time_colored, dur_colored, flow.cyan());
                } else {
                    println!("{} {} {}", time_display, dur_str, flow);
                }
            }
        }
    }
}

fn format_tool_flow(tools: &[crate::core::ToolSummary], enable_color: bool) -> String {
    let parts: Vec<String> = tools
        .iter()
        .map(|tool| {
            let status_indicator = if tool.is_error {
                if enable_color {
                    format!("{} ", "✗".red())
                } else {
                    "✗ ".to_string()
                }
            } else if enable_color {
                format!("{} ", "✓".green())
            } else {
                "✓ ".to_string()
            };

            let formatted = if !tool.input_summary.is_empty() {
                if tool.count > 1 {
                    format!("{} x{}", tool.input_summary, tool.count)
                } else {
                    tool.input_summary.clone()
                }
            } else if tool.count > 1 {
                format!("x{}", tool.count)
            } else {
                String::new()
            };

            let tool_display = if formatted.is_empty() {
                tool.name.clone()
            } else {
                format!("{}({})", tool.name, formatted)
            };

            format!("{}{}", status_indicator, tool_display)
        })
        .collect();

    parts.join(" → ")
}

fn print_session_summary(events: &[AgentEventV1], enable_color: bool) {
    if events.is_empty() {
        return;
    }

    if enable_color {
        println!("{}", format!("{}", "---".bright_black()));
        println!(
            "{}",
            format!("{}", "Session Summary:".bright_white().bold())
        );
    } else {
        println!("---");
        println!("Session Summary:");
    }

    // Count events by type
    let mut user_count = 0;
    let mut assistant_count = 0;
    let mut tool_call_count = 0;
    let mut reasoning_count = 0;
    let mut file_ops = std::collections::HashMap::new();

    // Calculate total tokens
    let mut total_input = 0u64;
    let mut total_output = 0u64;
    let mut total_cached = 0u64;
    let mut total_thinking = 0u64;

    for event in events {
        match event.event_type {
            EventType::UserMessage => user_count += 1,
            EventType::AssistantMessage => assistant_count += 1,
            EventType::ToolCall => tool_call_count += 1,
            EventType::Reasoning => reasoning_count += 1,
            _ => {}
        }

        if let Some(file_op) = &event.file_op {
            *file_ops.entry(*file_op).or_insert(0) += 1;
        }

        if let Some(t) = event.tokens_input {
            total_input += t;
        }
        if let Some(t) = event.tokens_output {
            total_output += t;
        }
        if let Some(t) = event.tokens_cached {
            total_cached += t;
        }
        if let Some(t) = event.tokens_thinking {
            total_thinking += t;
        }
    }

    if enable_color {
        println!(
            "  {}: {}",
            format!("{}", "Events".cyan()),
            format!("{}", events.len().to_string().bright_white())
        );
        println!(
            "    User messages: {}",
            format!("{}", user_count.to_string().green())
        );
        println!(
            "    Assistant messages: {}",
            format!("{}", assistant_count.to_string().blue())
        );
        println!(
            "    Tool calls: {}",
            format!("{}", tool_call_count.to_string().yellow())
        );
        println!(
            "    Reasoning blocks: {}",
            format!("{}", reasoning_count.to_string().cyan())
        );
    } else {
        println!("  Events: {}", events.len());
        println!("    User messages: {}", user_count);
        println!("    Assistant messages: {}", assistant_count);
        println!("    Tool calls: {}", tool_call_count);
        println!("    Reasoning blocks: {}", reasoning_count);
    }

    if !file_ops.is_empty() {
        if enable_color {
            println!("  {}:", format!("{}", "File operations".cyan()));
        } else {
            println!("  File operations:");
        }
        for (op, count) in file_ops.iter() {
            if enable_color {
                println!(
                    "    {}: {}",
                    op,
                    format!("{}", count.to_string().bright_white())
                );
            } else {
                println!("    {}: {}", op, count);
            }
        }
    }

    let total_tokens = total_input + total_output;
    if total_tokens > 0 {
        if enable_color {
            println!(
                "  {}: {}",
                format!("{}", "Tokens".cyan()),
                format!("{}", total_tokens.to_string().bright_white())
            );
            println!(
                "    Input: {}",
                format!("{}", total_input.to_string().bright_white())
            );
            println!(
                "    Output: {}",
                format!("{}", total_output.to_string().bright_white())
            );
            if total_cached > 0 {
                println!(
                    "    Cached: {}",
                    format!("{}", total_cached.to_string().bright_yellow())
                );
            }
            if total_thinking > 0 {
                println!(
                    "    Thinking: {}",
                    format!("{}", total_thinking.to_string().bright_cyan())
                );
            }
        } else {
            println!("  Tokens: {}", total_tokens);
            println!("    Input: {}", total_input);
            println!("    Output: {}", total_output);
            if total_cached > 0 {
                println!("    Cached: {}", total_cached);
            }
            if total_thinking > 0 {
                println!("    Thinking: {}", total_thinking);
            }
        }
    }

    // Calculate duration
    if let (Some(first), Some(last)) = (events.first(), events.last()) {
        if let (Ok(start), Ok(end)) = (
            DateTime::parse_from_rfc3339(&first.ts),
            DateTime::parse_from_rfc3339(&last.ts),
        ) {
            let duration = end.signed_duration_since(start);
            let minutes = duration.num_minutes();
            let seconds = duration.num_seconds() % 60;
            if enable_color {
                println!(
                    "  {}: {}m {}s",
                    format!("{}", "Duration".cyan()),
                    minutes,
                    seconds
                );
            } else {
                println!("  Duration: {}m {}s", minutes, seconds);
            }
        }
    }
}
