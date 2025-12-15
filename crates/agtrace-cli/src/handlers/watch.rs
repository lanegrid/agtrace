use crate::streaming::{SessionWatcher, StreamEvent};
use agtrace_types::v2::{AgentEvent, EventPayload};
use anyhow::Result;
use chrono::Local;
use owo_colors::OwoColorize;
use std::path::Path;

/// Handle the watch command - auto-attach to latest session and stream formatted events
pub fn handle(log_root: &Path) -> Result<()> {
    println!("{} {}", "[👀 Watching]".bright_cyan(), log_root.display());

    // Create session watcher
    let watcher = SessionWatcher::new(log_root.to_path_buf())?;
    let rx = watcher.into_receiver();

    // Event loop - receive and display events
    for event in rx {
        match event {
            StreamEvent::Attached { path, .. } => {
                println!(
                    "{}  {}\n",
                    "✨ Attached to active session:".bright_green(),
                    path.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| path.display().to_string())
                );
            }
            StreamEvent::NewEvents(events) => {
                for event in events {
                    print_event(&event);
                }
            }
            StreamEvent::SessionRotated { new_path, .. } => {
                println!(
                    "\n{} {}\n",
                    "✨ New session detected:".bright_green(),
                    new_path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| new_path.display().to_string())
                );
            }
            StreamEvent::Error(msg) => {
                eprintln!("{} {}", "❌ Error:".red(), msg);
            }
        }
    }

    Ok(())
}

/// Print a formatted event to stdout
fn print_event(event: &AgentEvent) {
    let time = event.timestamp.with_timezone(&Local).format("%H:%M:%S");

    match &event.payload {
        EventPayload::User(payload) => {
            let text = truncate(&payload.text, 100);
            println!("{} {} \"{}\"", time.dimmed(), "👤 User:".bold(), text);
        }
        EventPayload::Reasoning(payload) => {
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

            // Check for safety alerts
            let alert = check_safety_alert(&payload.arguments);

            let colored_name = color_fn(&payload.name);
            println!("{} {} {}: {}", time.dimmed(), icon, colored_name, summary);
            if let Some(warning) = alert {
                println!("             {} {}", "↳ ⚠️  ALERT:".red(), warning.red());
            }
        }
        EventPayload::ToolResult(payload) => {
            if payload.is_error {
                let output = truncate(&payload.output, 100);
                println!("{} {} {}", time.dimmed(), "❌ Fail:".red(), output.red());
            }
            // Success results are not shown (too noisy for MVP)
        }
        EventPayload::Message(payload) => {
            let text = truncate(&payload.text, 100);
            println!("{} {} {}", time.dimmed(), "💬 Msg:".cyan(), text);
        }
        EventPayload::TokenUsage(_) => {
            // Skip token usage (sidecar info, not relevant for stream)
        }
    }
}

/// Categorize a tool by name and return (icon, color_fn)
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

/// Format tool call arguments into a concise summary
fn format_tool_call(_name: &str, args: &serde_json::Value) -> String {
    // Extract key arguments based on common patterns
    if let Some(obj) = args.as_object() {
        // Common argument names to look for
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

    // Fallback: show first 40 chars of JSON
    let json = args.to_string();
    format!("({})", truncate(&json, 40))
}

/// Check for potentially dangerous operations
fn check_safety_alert(args: &serde_json::Value) -> Option<String> {
    if let Some(obj) = args.as_object() {
        // Check for path traversal
        for (_key, value) in obj.iter() {
            if let Some(s) = value.as_str() {
                if s.contains("..") {
                    return Some("Path contains '..' (outside access)".to_string());
                }
                if s.starts_with('/') && !s.starts_with("/Users/") && !s.starts_with("/home/") {
                    return Some("Absolute path outside user directory".to_string());
                }
            }
        }
    }
    None
}

/// Truncate text to max length with ellipsis
fn truncate(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        format!("{}...", &text[..max_len])
    }
}
