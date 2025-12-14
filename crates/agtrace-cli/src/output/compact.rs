use agtrace_engine::AgentSession;
use agtrace_types::ToolStatus;
use owo_colors::OwoColorize;

/// Options for compact format output
#[derive(Debug, Clone)]
pub struct CompactFormatOpts {
    pub enable_color: bool,
    pub relative_time: bool,
}

impl Default for CompactFormatOpts {
    fn default() -> Self {
        Self {
            enable_color: true,
            relative_time: true,
        }
    }
}

fn format_time_utc(
    session_start: Option<chrono::DateTime<chrono::Utc>>,
    _text: &str,
    timestamp: chrono::DateTime<chrono::Utc>,
) -> String {
    if let Some(start) = session_start {
        let duration = timestamp.signed_duration_since(start);
        let seconds = duration.num_seconds();
        if seconds < 60 {
            format!("[+{:02}:{:02}]", 0, seconds)
        } else {
            let minutes = seconds / 60;
            let secs = seconds % 60;
            format!("[+{:02}:{:02}]", minutes, secs)
        }
    } else {
        let ts_str = timestamp.to_rfc3339();
        if ts_str.len() >= 19 {
            format!("[{}]", &ts_str[11..19])
        } else {
            "[+00:00]".to_string()
        }
    }
}

fn format_duration(duration_ms: u64) -> String {
    if duration_ms >= 1000 {
        format!("{:2}s    ", duration_ms / 1000)
    } else if duration_ms > 0 {
        format!("{:3}ms  ", duration_ms)
    } else {
        "   -   ".to_string()
    }
}

/// Format AgentSession into compact string representation
pub fn format_session_compact(session: &AgentSession, opts: &CompactFormatOpts) -> Vec<String> {
    if session.turns.is_empty() {
        let msg = "No turns to display";
        return vec![if opts.enable_color {
            format!("{}", msg.bright_black())
        } else {
            msg.to_string()
        }];
    }

    let session_start = if opts.relative_time {
        Some(session.start_time)
    } else {
        None
    };

    let mut lines = Vec::new();

    for turn in &session.turns {
        // User message
        let time_display = format_time_utc(
            session_start,
            turn.user.content.text.as_str(),
            turn.timestamp,
        );
        let dur_placeholder = "   -   ";
        let line = if opts.enable_color {
            format!(
                "{} {} User: \"{}\"",
                time_display.bright_black(),
                dur_placeholder.bright_black(),
                turn.user.content.text.green()
            )
        } else {
            format!(
                "{} {} User: \"{}\"",
                time_display, dur_placeholder, turn.user.content.text
            )
        };
        lines.push(line);

        // Steps
        for step in &turn.steps {
            // Tool executions
            if !step.tools.is_empty() {
                let duration_ms = step
                    .tools
                    .iter()
                    .filter_map(|t| t.duration_ms)
                    .max()
                    .unwrap_or(0);
                let dur_str = format_duration(duration_ms as u64);

                let tools_display = format_tool_executions(&step.tools, opts.enable_color);

                let time_display = step
                    .tools
                    .first()
                    .map(|t| format_time_utc(session_start, "", t.call.timestamp))
                    .unwrap_or_else(|| "   -   ".to_string());

                let line = if opts.enable_color {
                    let dur_colored = if duration_ms > 30000 {
                        format!("{}", dur_str.red())
                    } else if duration_ms > 10000 {
                        format!("{}", dur_str.yellow())
                    } else {
                        format!("{}", dur_str.bright_black())
                    };
                    format!(
                        "{} {} {}",
                        time_display.bright_black(),
                        dur_colored,
                        tools_display.cyan()
                    )
                } else {
                    format!("{} {} {}", time_display, dur_str, tools_display)
                };
                lines.push(line);
            }

            // Assistant messages (if any meaningful text)
            if let Some(msg) = &step.message {
                if !msg.content.text.trim().is_empty() {
                    let time_display = format_time_utc(session_start, "", step.timestamp);
                    let dur_placeholder = "   -   ";
                    let text_preview = if msg.content.text.chars().count() > 100 {
                        let chars: Vec<char> = msg.content.text.chars().take(100).collect();
                        chars.iter().collect::<String>() + "..."
                    } else {
                        msg.content.text.clone()
                    };
                    let line = if opts.enable_color {
                        format!(
                            "{} {} Assistant: \"{}\"",
                            time_display.bright_black(),
                            dur_placeholder.bright_black(),
                            text_preview.blue()
                        )
                    } else {
                        format!(
                            "{} {} Assistant: \"{}\"",
                            time_display, dur_placeholder, text_preview
                        )
                    };
                    lines.push(line);
                }
            }
        }
    }

    lines
}

fn format_tool_executions(tools: &[agtrace_engine::ToolExecution], enable_color: bool) -> String {
    let parts: Vec<String> = tools
        .iter()
        .map(|tool_exec| {
            let status = if tool_exec.is_error {
                Some(ToolStatus::Error)
            } else if tool_exec.result.is_some() {
                Some(ToolStatus::Success)
            } else {
                None
            };
            let status_indicator = format_tool_status(status.as_ref(), enable_color);

            let input_summary = extract_input_summary(&tool_exec.call.content);
            let tool_display = if !input_summary.is_empty() {
                format!("{}({})", tool_exec.call.content.name, input_summary)
            } else {
                tool_exec.call.content.name.clone()
            };
            format!("{}{}", status_indicator, tool_display)
        })
        .collect();

    if parts.is_empty() {
        "no tools".to_string()
    } else {
        parts.join(" → ")
    }
}

fn extract_input_summary(payload: &agtrace_types::v2::ToolCallPayload) -> String {
    // Try to extract meaningful summary from arguments
    if let Some(file_path) = payload.arguments.get("file_path").and_then(|v| v.as_str()) {
        if let Some(filename) = std::path::Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
        {
            return filename.to_string();
        }
    }

    if let Some(cmd) = payload.arguments.get("command").and_then(|v| v.as_str()) {
        return truncate_string(cmd, 50);
    }

    if let Some(pattern) = payload.arguments.get("pattern").and_then(|v| v.as_str()) {
        return format!("\"{}\"", truncate_string(pattern, 30));
    }

    String::new()
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let chars: Vec<char> = s.chars().take(max_len - 3).collect();
        format!("{}...", chars.iter().collect::<String>())
    }
}

fn format_tool_status(status: Option<&ToolStatus>, enable_color: bool) -> String {
    match status {
        Some(ToolStatus::Success) => {
            if enable_color {
                format!("{} ", "✓".green())
            } else {
                "✓ ".to_string()
            }
        }
        Some(ToolStatus::Error) => {
            if enable_color {
                format!("{} ", "✗".red())
            } else {
                "✗ ".to_string()
            }
        }
        Some(ToolStatus::InProgress) => {
            if enable_color {
                format!("{} ", "⏳".yellow())
            } else {
                "⏳ ".to_string()
            }
        }
        Some(ToolStatus::Unknown) | None => {
            if enable_color {
                format!("{} ", "?".bright_black())
            } else {
                "? ".to_string()
            }
        }
    }
}
