use agtrace_engine::{ActionResult, ChainItem, Span, Turn};
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

/// Format turns into compact string representation
pub fn format_turns_compact(turns: &[Turn], opts: &CompactFormatOpts) -> Vec<String> {
    if turns.is_empty() {
        let msg = "No events to display";
        return vec![if opts.enable_color {
            format!("{}", msg.bright_black())
        } else {
            msg.to_string()
        }];
    }

    let session_start = if opts.relative_time {
        turns
            .first()
            .and_then(|t| DateTime::parse_from_rfc3339(t.timestamp()).ok())
    } else {
        None
    };

    let mut lines = Vec::new();

    for turn in turns {
        let line = match turn {
            Turn::User { timestamp, content } => {
                let time_display = format_time(session_start, timestamp);
                let dur_placeholder = "   -   ";

                if opts.enable_color {
                    format!(
                        "{} {} User: \"{}\"",
                        time_display.bright_black(),
                        dur_placeholder.bright_black(),
                        content.green()
                    )
                } else {
                    format!("{} {} User: \"{}\"", time_display, dur_placeholder, content)
                }
            }

            Turn::Agent {
                timestamp,
                chain,
                stats,
                ..
            } => {
                let time_display = format_time(session_start, timestamp);
                let duration_ms = stats.duration_ms.unwrap_or(0);
                let dur_str = format_duration(duration_ms);
                let chain_display = format_chain(chain, opts.enable_color);

                if opts.enable_color {
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
                        chain_display.cyan()
                    )
                } else {
                    format!("{} {} {}", time_display, dur_str, chain_display)
                }
            }

            Turn::System {
                timestamp,
                message,
                kind,
            } => {
                let time_display = format_time(session_start, timestamp);
                let dur_placeholder = "   -   ";
                let kind_str = format!("{:?}", kind);

                if opts.enable_color {
                    format!(
                        "{} {} {}: \"{}\"",
                        time_display.bright_black(),
                        dur_placeholder.bright_black(),
                        kind_str.yellow(),
                        message.white()
                    )
                } else {
                    format!(
                        "{} {} {}: \"{}\"",
                        time_display, dur_placeholder, kind_str, message
                    )
                }
            }
        };
        lines.push(line);
    }

    lines
}

pub fn print_turns_compact(turns: &[Turn], enable_color: bool) {
    let opts = CompactFormatOpts {
        enable_color,
        relative_time: true,
    };
    let lines = format_turns_compact(turns, &opts);
    for line in lines {
        println!("{}", line);
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

fn format_chain(chain: &[ChainItem], enable_color: bool) -> String {
    let parts: Vec<String> = chain
        .iter()
        .filter_map(|item| match item {
            ChainItem::Thought { .. } => None,
            ChainItem::Action {
                tool_name,
                input_summary,
                result,
            } => {
                let status_indicator = format_action_result(result, enable_color);
                let tool_display = if !input_summary.is_empty() {
                    format!("{}({})", tool_name, input_summary)
                } else {
                    tool_name.clone()
                };
                Some(format!("{}{}", status_indicator, tool_display))
            }
        })
        .collect();

    if parts.is_empty() {
        "thinking...".to_string()
    } else {
        parts.join(" → ")
    }
}

fn format_action_result(result: &ActionResult, enable_color: bool) -> String {
    match result {
        ActionResult::Success { .. } => {
            if enable_color {
                format!("{} ", "✓".green())
            } else {
                "✓ ".to_string()
            }
        }
        ActionResult::Failure { .. } => {
            if enable_color {
                format!("{} ", "✗".red())
            } else {
                "✗ ".to_string()
            }
        }
        ActionResult::Denied { .. } => {
            if enable_color {
                format!("{} ", "⊘".yellow())
            } else {
                "⊘ ".to_string()
            }
        }
        ActionResult::Interrupted => {
            if enable_color {
                format!("{} ", "⏸".yellow())
            } else {
                "⏸ ".to_string()
            }
        }
        ActionResult::Missing => {
            if enable_color {
                format!("{} ", "?".bright_black())
            } else {
                "? ".to_string()
            }
        }
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
                let text_preview = if assistant.text.len() > 100 {
                    format!("{}...", &assistant.text[..100])
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
