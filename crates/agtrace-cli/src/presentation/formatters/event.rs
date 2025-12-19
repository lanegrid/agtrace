use super::FormatOptions;
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
                let text = truncate(&payload.text, 100);
                if self.options.enable_color {
                    write!(
                        f,
                        "{} {} [T{}] \"{}\"",
                        time_display,
                        "👤 User:".bold(),
                        self.turn_context + 1,
                        text
                    )
                } else {
                    write!(
                        f,
                        "{} User [T{}] \"{}\"",
                        time_display,
                        self.turn_context + 1,
                        text
                    )
                }
            }
            EventPayload::Reasoning(payload) => {
                if payload.text.trim().is_empty() {
                    return Ok(());
                }
                let text = truncate(&payload.text, 50);
                if self.options.enable_color {
                    write!(
                        f,
                        "{} {} {}",
                        time_display,
                        "🧠 Thnk:".dimmed(),
                        text.dimmed()
                    )
                } else {
                    write!(f, "{} Thnk: {}", time_display, text)
                }
            }
            EventPayload::ToolCall(payload) => {
                let (icon, name_str) = if self.options.enable_color {
                    let (icon, color_fn) = categorize_tool(&payload.name);
                    (icon, color_fn(&payload.name))
                } else {
                    (categorize_tool(&payload.name).0, payload.name.clone())
                };
                let summary =
                    format_tool_call(&payload.name, &payload.arguments, self.project_root);

                write!(f, "{} {} {}: {}", time_display, icon, name_str, summary)
            }
            EventPayload::ToolResult(payload) => {
                if payload.is_error {
                    let output = truncate(&payload.output, 100);
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
                let text = truncate(&payload.text, 100);
                if self.options.enable_color {
                    write!(f, "{} {} {}", time_display, "💬 Msg:".cyan(), text)
                } else {
                    write!(f, "{} Msg: {}", time_display, text)
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
                    let text = truncate(&payload.text, 100);
                    (icon, color_fn(&text))
                } else {
                    let text = truncate(&payload.text, 100);
                    ("Info:", text)
                };
                write!(f, "{} {} {}", time_display, icon, colored_text)
            }
        }
    }
}

fn categorize_tool(name: &str) -> (&'static str, fn(&str) -> String) {
    let lower = name.to_lowercase();

    if lower.contains("read")
        || lower.contains("ls")
        || lower.contains("cat")
        || lower.contains("grep")
        || lower.contains("search")
        || lower.contains("view")
    {
        ("📖", |s: &str| s.cyan().to_string())
    } else if lower.contains("write") || lower.contains("edit") || lower.contains("replace") {
        ("🛠️", |s: &str| s.yellow().to_string())
    } else if lower.contains("run")
        || lower.contains("exec")
        || lower.contains("bash")
        || lower.contains("python")
        || lower.contains("test")
    {
        ("🧪", |s: &str| s.magenta().to_string())
    } else {
        ("🔧", |s: &str| s.white().to_string())
    }
}

fn format_tool_call(
    _name: &str,
    args: &serde_json::Value,
    project_root: Option<&std::path::Path>,
) -> String {
    if let Some(obj) = args.as_object() {
        if let Some(path) = obj.get("path").or_else(|| obj.get("file_path")) {
            if let Some(path_str) = path.as_str() {
                let shortened = shorten_path(path_str, project_root);
                return format!("(\"{}\")", truncate(&shortened, 60));
            }
        }
        if let Some(command) = obj.get("command") {
            if let Some(cmd_str) = command.as_str() {
                return format!("(\"{}\")", truncate(cmd_str, 60));
            }
        }
        if let Some(pattern) = obj.get("pattern") {
            if let Some(pat_str) = pattern.as_str() {
                return format!("(\"{}\")", truncate(pat_str, 60));
            }
        }
    }

    let json = args.to_string();
    format!("({})", truncate(&json, 40))
}

fn truncate(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        let mut boundary = max_len;
        while boundary > 0 && !text.is_char_boundary(boundary) {
            boundary -= 1;
        }
        format!("{}...", &text[..boundary])
    }
}

/// Convert absolute path to relative path if it's shorter
fn shorten_path(path: &str, project_root: Option<&std::path::Path>) -> String {
    if let Some(root) = project_root {
        if let Ok(relative) = std::path::Path::new(path).strip_prefix(root) {
            let relative_str = relative.to_string_lossy();
            // Use relative path only if it's shorter
            if relative_str.len() < path.len() {
                return relative_str.to_string();
            }
        }
    }
    path.to_string()
}
