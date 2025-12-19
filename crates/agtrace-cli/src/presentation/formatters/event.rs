use super::{text, tool, FormatOptions};
use agtrace_types::{AgentEvent, EventPayload};
use chrono::{DateTime, Local, Utc};
use owo_colors::OwoColorize;
use std::fmt;

/// View for displaying a single AgentEvent
pub struct EventView<'a> {
    pub event: &'a AgentEvent,
    pub options: &'a FormatOptions,
    /// Start time for relative time calculation
    pub session_start: Option<DateTime<Utc>>,
    /// Turn context (0-indexed)
    pub turn_context: usize,
    /// Project root for path shortening
    pub project_root: Option<&'a std::path::Path>,
}

impl<'a> fmt::Display for EventView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let time = if let Some(start) = self.session_start {
            let duration = self.event.timestamp.signed_duration_since(start);
            let seconds = duration.num_seconds();
            if seconds < 60 {
                format!("[+{:02}s  ]", seconds)
            } else {
                let minutes = seconds / 60;
                let secs = seconds % 60;
                format!("[+{}m {:02}s]", minutes, secs)
            }
        } else {
            let ts = self
                .event
                .timestamp
                .with_timezone(&Local)
                .format("%H:%M:%S");
            format!("[{}]", ts)
        };

        let time_display = if self.options.enable_color {
            format!("{}", time.bright_black())
        } else {
            time
        };

        match &self.event.payload {
            EventPayload::User(payload) => {
                if payload.text.trim().is_empty() {
                    return Ok(());
                }
                let txt = text::truncate(&payload.text, 100);
                if self.options.enable_color {
                    write!(
                        f,
                        "{} {} [T{}] \"{}\"",
                        time_display,
                        "👤 User:".bold(),
                        self.turn_context + 1,
                        txt
                    )
                } else {
                    write!(
                        f,
                        "{} User [T{}] \"{}\"",
                        time_display,
                        self.turn_context + 1,
                        txt
                    )
                }
            }
            EventPayload::Reasoning(payload) => {
                if payload.text.trim().is_empty() {
                    return Ok(());
                }
                let txt = text::truncate(&payload.text, 50);
                if self.options.enable_color {
                    write!(
                        f,
                        "{} {} {}",
                        time_display,
                        "🧠 Thnk:".dimmed(),
                        txt.dimmed()
                    )
                } else {
                    write!(f, "{} Thnk: {}", time_display, txt)
                }
            }
            EventPayload::ToolCall(payload) => {
                let (icon, name_str) = if self.options.enable_color {
                    let (icon, color_fn) = tool::categorize_tool(&payload.name);
                    (icon, color_fn(&payload.name))
                } else {
                    (tool::categorize_tool(&payload.name).0, payload.name.clone())
                };
                let summary =
                    tool::format_tool_call(&payload.name, &payload.arguments, self.project_root);

                write!(f, "{} {} {}: {}", time_display, icon, name_str, summary)
            }
            EventPayload::ToolResult(payload) => {
                if payload.is_error {
                    let output = text::truncate(&payload.output, 100);
                    if self.options.enable_color {
                        write!(f, "{} {} {}", time_display, "❌ Fail:".red(), output.red())
                    } else {
                        write!(f, "{} Fail: {}", time_display, output)
                    }
                } else {
                    Ok(())
                }
            }
            EventPayload::Message(payload) => {
                if payload.text.trim().is_empty() {
                    return Ok(());
                }
                let txt = text::truncate(&payload.text, 100);
                if self.options.enable_color {
                    write!(f, "{} {} {}", time_display, "💬 Msg:".cyan(), txt)
                } else {
                    write!(f, "{} Msg: {}", time_display, txt)
                }
            }
            EventPayload::TokenUsage(_) => Ok(()),
            EventPayload::Notification(payload) => {
                let (icon, colored_text) = if self.options.enable_color {
                    let (icon, color_fn): (&str, fn(&str) -> String) =
                        match payload.level.as_deref() {
                            Some("warning") => ("⚠️", |s: &str| s.yellow().to_string()),
                            Some("error") => ("❌", |s: &str| s.red().to_string()),
                            _ => ("ℹ️", |s: &str| s.cyan().to_string()),
                        };
                    let txt = text::truncate(&payload.text, 100);
                    (icon, color_fn(&txt))
                } else {
                    let txt = text::truncate(&payload.text, 100);
                    ("Info:", txt)
                };
                write!(f, "{} {} {}", time_display, icon, colored_text)
            }
        }
    }
}
