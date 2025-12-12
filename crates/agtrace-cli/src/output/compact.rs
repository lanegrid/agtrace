use agtrace_engine::{Activity, ToolSummary};
use agtrace_types::Role;
use chrono::DateTime;
use owo_colors::OwoColorize;

pub fn print_activities_compact(activities: &[Activity], enable_color: bool) {
    if activities.is_empty() {
        let msg = "No events to display";
        if enable_color {
            println!("{}", msg.bright_black());
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

fn format_tool_flow(tools: &[ToolSummary], enable_color: bool) -> String {
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
