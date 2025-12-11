#![allow(clippy::format_in_format_args)] // Intentional for colored terminal output

use crate::db::Database;
use crate::model::{AgentEventV1, EventType};
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
        print_events_compact(&filtered_events, enable_color);
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

        let event_type_str = if enable_color {
            match event.event_type {
                EventType::UserMessage => format!("{}", event_type_name.green()),
                EventType::AssistantMessage => format!("{}", event_type_name.blue()),
                EventType::Reasoning => format!("{}", event_type_name.cyan()),
                EventType::ToolCall => format!("{}", event_type_name.yellow()),
                EventType::ToolResult => format!("{}", event_type_name.magenta()),
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

        println!("{} {:<20} {}", time_colored, event_type_str, role_str);

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

struct ToolInfo {
    name: String,
    target: Option<String>,
}

struct ToolChainBuffer {
    tools: Vec<ToolInfo>,
    start_ts: Option<DateTime<chrono::FixedOffset>>,
    end_ts: Option<DateTime<chrono::FixedOffset>>,
}

impl ToolChainBuffer {
    fn new() -> Self {
        Self {
            tools: Vec::new(),
            start_ts: None,
            end_ts: None,
        }
    }

    fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    fn mark_start(&mut self, ts: &str) {
        if let Ok(parsed_ts) = DateTime::parse_from_rfc3339(ts) {
            if self.start_ts.is_none() {
                self.start_ts = Some(parsed_ts);
            }
        }
    }

    fn push_tool(
        &mut self,
        tool_name: Option<&String>,
        file_path: Option<&String>,
        text: Option<&String>,
        ts: &str,
    ) {
        if let Some(name) = tool_name {
            let target = extract_target_summary(name, file_path, text);

            self.tools.push(ToolInfo {
                name: name.clone(),
                target,
            });
        }
        if let Ok(parsed_ts) = DateTime::parse_from_rfc3339(ts) {
            if self.start_ts.is_none() {
                self.start_ts = Some(parsed_ts);
            }
            self.end_ts = Some(parsed_ts);
        }
    }

    fn mark_end(&mut self, ts: &str) {
        if let Ok(parsed_ts) = DateTime::parse_from_rfc3339(ts) {
            self.end_ts = Some(parsed_ts);
        }
    }

    fn format_flow(&self) -> String {
        if self.tools.is_empty() {
            return String::new();
        }

        let mut parts = Vec::new();
        let mut i = 0;

        while i < self.tools.len() {
            let current_tool = &self.tools[i].name;
            let mut targets = Vec::new();

            while i < self.tools.len() && &self.tools[i].name == current_tool {
                targets.push(self.tools[i].target.clone());
                i += 1;
            }

            let formatted = format_tool_with_targets(current_tool, &targets);
            parts.push(formatted);
        }

        parts.join(" → ")
    }

    fn flush_and_print(
        &mut self,
        session_start: Option<DateTime<chrono::FixedOffset>>,
        enable_color: bool,
    ) {
        if self.is_empty() {
            return;
        }

        let flow = self.format_flow();

        let duration_ms = if let (Some(start), Some(end)) = (self.start_ts, self.end_ts) {
            end.signed_duration_since(start).num_milliseconds() as u64
        } else {
            0
        };

        let time_display = if let (Some(session_start), Some(chain_start)) = (session_start, self.start_ts) {
            let duration = chain_start.signed_duration_since(session_start);
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

        let dur_str = if duration_ms >= 1000 {
            format!("{:2}s    ", duration_ms / 1000)
        } else if duration_ms > 0 {
            format!("{:3}ms  ", duration_ms)
        } else {
            "       ".to_string()
        };

        let dur_colored = if enable_color {
            if duration_ms > 30000 {
                format!("{}", dur_str.red())
            } else if duration_ms > 10000 {
                format!("{}", dur_str.yellow())
            } else {
                format!("{}", dur_str.bright_black())
            }
        } else {
            dur_str.clone()
        };

        if enable_color {
            println!("{} {} {}", time_colored, dur_colored, flow.cyan());
        } else {
            println!("{} {} {}", time_display, dur_str, flow);
        }

        self.tools.clear();
        self.start_ts = None;
        self.end_ts = None;
    }
}

fn extract_target_summary(
    tool_name: &str,
    file_path: Option<&String>,
    text: Option<&String>,
) -> Option<String> {
    match tool_name {
        "Read" | "Edit" | "Write" => {
            file_path.and_then(|p| {
                std::path::Path::new(p)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string())
            })
        }
        "Bash" => {
            text.and_then(|t| {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(t) {
                    json.get("command")
                        .and_then(|v| v.as_str())
                        .map(|cmd| {
                            let cmd = cmd.trim();
                            if cmd.len() > 30 {
                                format!("{}...", &cmd[..27])
                            } else {
                                cmd.to_string()
                            }
                        })
                } else {
                    let cmd = t.trim();
                    if cmd.len() > 30 {
                        Some(format!("{}...", &cmd[..27]))
                    } else {
                        Some(cmd.to_string())
                    }
                }
            })
        }
        "Glob" => {
            text.and_then(|t| {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(t) {
                    json.get("pattern")
                        .and_then(|v| v.as_str())
                        .map(|p| {
                            if p.len() > 20 {
                                format!("\"{}...\"", &p[..17])
                            } else {
                                format!("\"{}\"", p)
                            }
                        })
                } else {
                    None
                }
            })
        }
        "Grep" => {
            text.and_then(|t| {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(t) {
                    json.get("pattern")
                        .and_then(|v| v.as_str())
                        .map(|p| {
                            if p.len() > 20 {
                                format!("\"{}...\"", &p[..17])
                            } else {
                                format!("\"{}\"", p)
                            }
                        })
                } else {
                    None
                }
            })
        }
        _ => None,
    }
}

fn format_tool_with_targets(tool_name: &str, targets: &[Option<String>]) -> String {
    let mut target_groups = Vec::new();
    let mut i = 0;

    while i < targets.len() {
        let current_target = &targets[i];
        let mut count = 1;

        while i + count < targets.len() && &targets[i + count] == current_target {
            count += 1;
        }

        if let Some(target) = current_target {
            if count > 1 {
                target_groups.push(format!("{} x{}", target, count));
            } else {
                target_groups.push(target.clone());
            }
        } else if count > 1 {
            target_groups.push(format!("x{}", count));
        }

        i += count;
    }

    if target_groups.is_empty() {
        tool_name.to_string()
    } else if target_groups.len() == 1 && target_groups[0].starts_with('x') {
        format!("{}({})", tool_name, target_groups[0])
    } else {
        format!("{}({})", tool_name, target_groups.join(", "))
    }
}

fn print_events_compact(events: &[AgentEventV1], enable_color: bool) {
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

    let mut buffer = ToolChainBuffer::new();

    for event in events {
        match event.event_type {
            EventType::UserMessage | EventType::AssistantMessage => {
                buffer.flush_and_print(session_start, enable_color);

                let time_display = if let (Some(start), Ok(current)) =
                    (session_start, DateTime::parse_from_rfc3339(&event.ts))
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
                    format!("[{}]", &event.ts[11..19])
                };

                let time_colored = if enable_color {
                    format!("{}", time_display.bright_black())
                } else {
                    time_display.clone()
                };

                let text = event.text.as_deref().unwrap_or("");
                // Flatten: replace newlines with spaces and normalize consecutive spaces
                let text_normalized = text.replace('\n', " ");
                let clean_text: String = text_normalized.split_whitespace().collect::<Vec<_>>().join(" ");

                // Display up to 100 chars to show concrete details beyond filler phrases
                let limit = 100;
                let preview: String = clean_text.chars().take(limit).collect();
                let text_display = if clean_text.len() > limit {
                    format!("{}...", preview)
                } else {
                    preview
                };

                let dur_placeholder = "   -   ";
                let dur_colored = if enable_color {
                    format!("{}", dur_placeholder.bright_black())
                } else {
                    dur_placeholder.to_string()
                };

                let role = if matches!(event.event_type, EventType::UserMessage) {
                    "User"
                } else {
                    "Asst"
                };

                if enable_color {
                    let colored_text = if matches!(event.event_type, EventType::UserMessage) {
                        format!("{}", text_display.green())
                    } else {
                        format!("{}", text_display.blue())
                    };
                    println!("{} {} {}: \"{}\"", time_colored, dur_colored, role, colored_text);
                } else {
                    println!("{} {} {}: \"{}\"", time_display, dur_placeholder, role, text_display);
                }
            }

            EventType::Reasoning => {
                buffer.mark_start(&event.ts);
            }

            EventType::ToolCall => {
                buffer.push_tool(
                    event.tool_name.as_ref(),
                    event.file_path.as_ref(),
                    event.text.as_ref(),
                    &event.ts,
                );
            }

            EventType::ToolResult => {
                buffer.mark_end(&event.ts);
            }

            _ => {}
        }
    }

    buffer.flush_and_print(session_start, enable_color);
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
            *file_ops.entry(file_op.clone()).or_insert(0) += 1;
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
