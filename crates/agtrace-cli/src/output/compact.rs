use agtrace_engine::{ActionResult, ChainItem, Turn};
use chrono::DateTime;
use owo_colors::OwoColorize;

pub fn print_turns_compact(turns: &[Turn], enable_color: bool) {
    if turns.is_empty() {
        let msg = "No events to display";
        if enable_color {
            println!("{}", msg.bright_black());
        } else {
            println!("{}", msg);
        }
        return;
    }

    let session_start = turns
        .first()
        .and_then(|t| DateTime::parse_from_rfc3339(t.timestamp()).ok());

    for turn in turns {
        match turn {
            Turn::User { timestamp, content } => {
                let time_display = format_time(session_start, timestamp);
                let dur_placeholder = "   -   ";

                if enable_color {
                    println!(
                        "{} {} User: \"{}\"",
                        time_display.bright_black(),
                        dur_placeholder.bright_black(),
                        content.green()
                    );
                } else {
                    println!("{} {} User: \"{}\"", time_display, dur_placeholder, content);
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
                let chain_display = format_chain(chain, enable_color);

                if enable_color {
                    let dur_colored = if duration_ms > 30000 {
                        format!("{}", dur_str.red())
                    } else if duration_ms > 10000 {
                        format!("{}", dur_str.yellow())
                    } else {
                        format!("{}", dur_str.bright_black())
                    };
                    println!(
                        "{} {} {}",
                        time_display.bright_black(),
                        dur_colored,
                        chain_display.cyan()
                    );
                } else {
                    println!("{} {} {}", time_display, dur_str, chain_display);
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

                if enable_color {
                    println!(
                        "{} {} {}: \"{}\"",
                        time_display.bright_black(),
                        dur_placeholder.bright_black(),
                        kind_str.yellow(),
                        message.white()
                    );
                } else {
                    println!(
                        "{} {} {}: \"{}\"",
                        time_display, dur_placeholder, kind_str, message
                    );
                }
            }
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
