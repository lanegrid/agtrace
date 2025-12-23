use crate::presentation::v1::formatters::{text, tool};
use crate::presentation::v1::view_models::{DisplayOptions, EventPayloadViewModel, EventViewModel};

type FormatOptions = DisplayOptions;
use chrono::{DateTime, Local, Utc};
use owo_colors::OwoColorize;
use std::fmt;

pub struct EventView<'a> {
    pub event: &'a EventViewModel,
    pub options: &'a FormatOptions,
    pub session_start: Option<DateTime<Utc>>,
    pub turn_context: usize,
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
            EventPayloadViewModel::User { text } => {
                if text.trim().is_empty() {
                    return Ok(());
                }
                let txt = text::truncate(text, 100);
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
            EventPayloadViewModel::Reasoning { text } => {
                if text.trim().is_empty() {
                    return Ok(());
                }
                let txt = text::truncate(text, 50);
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
            EventPayloadViewModel::ToolCall { name, arguments } => {
                let (icon, name_str) = if self.options.enable_color {
                    let (icon, color_fn) = tool::categorize_tool(name);
                    (icon, color_fn(name))
                } else {
                    (tool::categorize_tool(name).0, name.clone())
                };
                let summary = tool::format_tool_call(name, arguments, None);

                write!(f, "{} {} {}: {}", time_display, icon, name_str, summary)
            }
            EventPayloadViewModel::ToolResult { output, is_error } => {
                if *is_error {
                    let output_text = text::truncate(output, 100);
                    if self.options.enable_color {
                        write!(
                            f,
                            "{} {} {}",
                            time_display,
                            "❌ Fail:".red(),
                            output_text.red()
                        )
                    } else {
                        write!(f, "{} Fail: {}", time_display, output_text)
                    }
                } else {
                    Ok(())
                }
            }
            EventPayloadViewModel::Message { text } => {
                if text.trim().is_empty() {
                    return Ok(());
                }
                let txt = text::truncate(text, 100);
                if self.options.enable_color {
                    write!(f, "{} {} {}", time_display, "💬 Msg:".cyan(), txt)
                } else {
                    write!(f, "{} Msg: {}", time_display, txt)
                }
            }
            EventPayloadViewModel::TokenUsage { .. } => Ok(()),
            EventPayloadViewModel::Notification { text, level } => {
                let (icon, colored_text) = if self.options.enable_color {
                    let (icon, color_fn): (&str, fn(&str) -> String) = match level.as_deref() {
                        Some("warning") => ("⚠️", |s: &str| s.yellow().to_string()),
                        Some("error") => ("❌", |s: &str| s.red().to_string()),
                        _ => ("ℹ️", |s: &str| s.cyan().to_string()),
                    };
                    let txt = text::truncate(text, 100);
                    (icon, color_fn(&txt))
                } else {
                    let txt = text::truncate(text, 100);
                    ("Info:", txt)
                };
                write!(f, "{} {} {}", time_display, icon, colored_text)
            }
        }
    }
}

pub struct TimelineView<'a> {
    pub events: &'a [EventViewModel],
    pub options: &'a FormatOptions,
}

impl<'a> fmt::Display for TimelineView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.events.is_empty() {
            return writeln!(f, "No events to display");
        }

        let session_start = self.events.first().map(|e| e.timestamp);
        let mut turn_count = 0;

        for event in self.events {
            if matches!(event.payload, EventPayloadViewModel::TokenUsage { .. }) {
                continue;
            }

            let view = EventView {
                event,
                options: self.options,
                session_start,
                turn_context: turn_count,
            };

            writeln!(f, "{}", view)?;

            if matches!(event.payload, EventPayloadViewModel::User { .. }) {
                turn_count += 1;
            }
        }

        Ok(())
    }
}
