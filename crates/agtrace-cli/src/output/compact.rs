use agtrace_engine::Span;
use agtrace_types::ToolStatus;
use chrono::DateTime;
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

fn format_time(session_start: Option<DateTime<chrono::FixedOffset>>, timestamp: &str) -> String {
    if let (Some(start), Ok(current)) = (session_start, DateTime::parse_from_rfc3339(timestamp)) {
        let duration = current.signed_duration_since(start);
        let seconds = duration.num_seconds();
        if seconds < 60 {
            format!("[+{:02}:{:02}]", 0, seconds)
        } else {
            let minutes = seconds / 60;
            let secs = seconds % 60;
            format!("[+{:02}:{:02}]", minutes, secs)
        }
    } else if timestamp.len() >= 19 {
        format!("[{}]", &timestamp[11..19])
    } else {
        "[+00:00]".to_string()
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

/// Format spans into compact string representation
pub fn format_spans_compact(spans: &[Span], opts: &CompactFormatOpts) -> Vec<String> {
    if spans.is_empty() {
        let msg = "No spans to display";
        return vec![if opts.enable_color {
            format!("{}", msg.bright_black())
        } else {
            msg.to_string()
        }];
    }

    let session_start = if opts.relative_time {
        spans
            .first()
            .and_then(|s| s.user.as_ref())
            .and_then(|u| DateTime::parse_from_rfc3339(&u.ts).ok())
    } else {
        None
    };

    let mut lines = Vec::new();

    for span in spans {
        // User message
        if let Some(user) = &span.user {
            let time_display = format_time(session_start, &user.ts);
            let dur_placeholder = "   -   ";
            let line = if opts.enable_color {
                format!(
                    "{} {} User: \"{}\"",
                    time_display.bright_black(),
                    dur_placeholder.bright_black(),
                    user.text.green()
                )
            } else {
                format!(
                    "{} {} User: \"{}\"",
                    time_display, dur_placeholder, user.text
                )
            };
            lines.push(line);
        }

        // Tool actions
        if !span.tools.is_empty() {
            let e2e_ms = span.stats.e2e_ms.unwrap_or(0);
            let dur_str = format_duration(e2e_ms);

            let tools_display = format_tools(&span.tools, opts.enable_color);

            let time_display = span
                .tools
                .first()
                .map(|t| format_time(session_start, &t.ts_call))
                .unwrap_or_else(|| "   -   ".to_string());

            let line = if opts.enable_color {
                let dur_colored = if e2e_ms > 30000 {
                    format!("{}", dur_str.red())
                } else if e2e_ms > 10000 {
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
        for assistant in &span.assistant {
            if !assistant.text.trim().is_empty() {
                let time_display = format_time(session_start, &assistant.ts);
                let dur_placeholder = "   -   ";
                let text_preview = if assistant.text.chars().count() > 100 {
                    let chars: Vec<char> = assistant.text.chars().take(100).collect();
                    chars.iter().collect::<String>() + "..."
                } else {
                    assistant.text.clone()
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

    lines
}

fn format_tools(tools: &[agtrace_engine::ToolAction], enable_color: bool) -> String {
    let parts: Vec<String> = tools
        .iter()
        .map(|tool| {
            let status_indicator = format_tool_status(tool.status.as_ref(), enable_color);
            let tool_display = if !tool.input_summary.is_empty() {
                format!("{}({})", tool.tool_name, tool.input_summary)
            } else {
                tool.tool_name.clone()
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
