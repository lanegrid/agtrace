use agtrace_types::{AgentEvent, EventPayload};
use chrono::{DateTime, Local, Utc};
use owo_colors::OwoColorize;

#[allow(dead_code)]
pub fn format_event(
    event: &AgentEvent,
    turn_context: usize,
    project_root: Option<&std::path::Path>,
) -> Option<String> {
    format_event_with_start(event, turn_context, project_root, None)
}

pub fn format_event_with_start(
    event: &AgentEvent,
    turn_context: usize,
    project_root: Option<&std::path::Path>,
    session_start: Option<DateTime<Utc>>,
) -> Option<String> {
    let time = if let Some(start) = session_start {
        let duration = event.timestamp.signed_duration_since(start);
        let seconds = duration.num_seconds();
        if seconds < 60 {
            format!("[+{:02}s  ]", seconds)
        } else {
            let minutes = seconds / 60;
            let secs = seconds % 60;
            format!("[+{}m {:02}s]", minutes, secs)
        }
    } else {
        let ts = event.timestamp.with_timezone(&Local).format("%H:%M:%S");
        format!("[{}]", ts)
    };

    let time_display = format!("{}", time.bright_black());

    match &event.payload {
        EventPayload::User(payload) => {
            if payload.text.trim().is_empty() {
                return None;
            }
            let text = truncate(&payload.text, 100);
            Some(format!(
                "{} {} [T{}] \"{}\"",
                time_display,
                "👤 User:".bold(),
                turn_context + 1,
                text
            ))
        }
        EventPayload::Reasoning(payload) => {
            if payload.text.trim().is_empty() {
                return None;
            }
            let text = truncate(&payload.text, 50);
            Some(format!(
                "{} {} {}",
                time_display,
                "🧠 Thnk:".dimmed(),
                text.dimmed()
            ))
        }
        EventPayload::ToolCall(payload) => {
            let (icon, color_fn) = categorize_tool(&payload.name);
            let summary = format_tool_call(&payload.name, &payload.arguments, project_root);

            let colored_name = color_fn(&payload.name);
            Some(format!(
                "{} {} {}: {}",
                time_display, icon, colored_name, summary
            ))
        }
        EventPayload::ToolResult(payload) => {
            if payload.is_error {
                let output = truncate(&payload.output, 100);
                Some(format!(
                    "{} {} {}",
                    time_display,
                    "❌ Fail:".red(),
                    output.red()
                ))
            } else {
                None
            }
        }
        EventPayload::Message(payload) => {
            if payload.text.trim().is_empty() {
                return None;
            }
            let text = truncate(&payload.text, 100);
            Some(format!("{} {} {}", time_display, "💬 Msg:".cyan(), text))
        }
        EventPayload::TokenUsage(_) => None,
        EventPayload::Notification(payload) => {
            let (icon, color_fn): (&str, fn(&str) -> String) = match payload.level.as_deref() {
                Some("warning") => ("⚠️", |s: &str| s.yellow().to_string()),
                Some("error") => ("❌", |s: &str| s.red().to_string()),
                _ => ("ℹ️", |s: &str| s.cyan().to_string()),
            };
            let text = truncate(&payload.text, 100);
            let colored_text = color_fn(&text);
            Some(format!("{} {} {}", time_display, icon, colored_text))
        }
    }
}

#[allow(dead_code)]
pub fn print_event(
    event: &AgentEvent,
    turn_context: usize,
    project_root: Option<&std::path::Path>,
) {
    if let Some(line) = format_event(event, turn_context, project_root) {
        println!("{}", line);
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
