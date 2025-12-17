use agtrace_types::v2::{AgentEvent, EventPayload};
use chrono::Local;
use owo_colors::OwoColorize;

pub fn print_event(event: &AgentEvent, turn_context: usize) {
    let time = event.timestamp.with_timezone(&Local).format("%H:%M:%S");

    match &event.payload {
        EventPayload::User(payload) => {
            if payload.text.trim().is_empty() {
                return;
            }
            let text = truncate(&payload.text, 100);
            println!(
                "{} {} [T{}] \"{}\"",
                time.dimmed(),
                "👤 User:".bold(),
                turn_context + 1,
                text
            );
        }
        EventPayload::Reasoning(payload) => {
            if payload.text.trim().is_empty() {
                return;
            }
            let text = truncate(&payload.text, 50);
            println!(
                "{} {} {}",
                time.dimmed(),
                "🧠 Thnk:".dimmed(),
                text.dimmed()
            );
        }
        EventPayload::ToolCall(payload) => {
            let (icon, color_fn) = categorize_tool(&payload.name);
            let summary = format_tool_call(&payload.name, &payload.arguments);

            let colored_name = color_fn(&payload.name);
            println!("{} {} {}: {}", time.dimmed(), icon, colored_name, summary);
        }
        EventPayload::ToolResult(payload) => {
            if payload.is_error {
                let output = truncate(&payload.output, 100);
                println!("{} {} {}", time.dimmed(), "❌ Fail:".red(), output.red());
            }
        }
        EventPayload::Message(payload) => {
            if payload.text.trim().is_empty() {
                return;
            }
            let text = truncate(&payload.text, 100);
            println!("{} {} {}", time.dimmed(), "💬 Msg:".cyan(), text);
        }
        EventPayload::TokenUsage(_) => {}
        EventPayload::Notification(payload) => {
            let (icon, color_fn): (&str, fn(&str) -> String) = match payload.level.as_deref() {
                Some("warning") => ("⚠️", |s: &str| s.yellow().to_string()),
                Some("error") => ("❌", |s: &str| s.red().to_string()),
                _ => ("ℹ️", |s: &str| s.cyan().to_string()),
            };
            let text = truncate(&payload.text, 100);
            let colored_text = color_fn(&text);
            println!("{} {} {}", time.dimmed(), icon, colored_text);
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

fn format_tool_call(_name: &str, args: &serde_json::Value) -> String {
    if let Some(obj) = args.as_object() {
        if let Some(path) = obj.get("path").or_else(|| obj.get("file_path")) {
            if let Some(path_str) = path.as_str() {
                return format!("(\"{}\")", truncate(path_str, 60));
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
