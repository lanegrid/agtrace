use crate::display_model::{DisplayOptions, TokenSummaryDisplay};
use crate::reactor::{Reaction, Reactor, ReactorContext};
use crate::token_limits::TokenLimits;
use agtrace_types::v2::{AgentEvent, EventPayload};
use anyhow::Result;
use chrono::Local;
use owo_colors::OwoColorize;

/// TUI Renderer - displays events to stdout
pub struct TuiRenderer {
    token_limits: TokenLimits,
}

impl TuiRenderer {
    pub fn new() -> Self {
        Self {
            token_limits: TokenLimits::new(),
        }
    }

    fn print_token_summary(&self, ctx: &ReactorContext) {
        let total = ctx.state.total_context_window_tokens() as u64;

        if total == 0 {
            return;
        }

        let limit = ctx.state.context_window_limit.or_else(|| {
            ctx.state
                .model
                .as_ref()
                .and_then(|m| self.token_limits.get_limit(m).map(|l| l.total_limit))
        });

        let summary = TokenSummaryDisplay {
            input: ctx.state.total_input_side_tokens(),
            output: ctx.state.total_output_side_tokens(),
            cache_creation: ctx.state.current_usage.cache_creation.0,
            cache_read: ctx.state.current_usage.cache_read.0,
            total: ctx.state.total_context_window_tokens(),
            limit,
            model: ctx.state.model.clone(),
        };

        let opts = DisplayOptions {
            enable_color: true,
            relative_time: false,
            truncate_text: None,
        };

        println!();
        let lines = crate::output::format_token_summary(&summary, &opts);
        for line in lines {
            println!("{}", line);
        }
    }
}

impl Reactor for TuiRenderer {
    fn name(&self) -> &str {
        "TuiRenderer"
    }

    fn handle(&mut self, ctx: ReactorContext) -> Result<Reaction> {
        let event = ctx.event;
        let turn_context = ctx.state.turn_count;

        print_event(event, turn_context);

        // Print token summary after TokenUsage events (when tokens are updated)
        if matches!(event.payload, EventPayload::TokenUsage(_)) {
            self.print_token_summary(&ctx);
        }

        Ok(Reaction::Continue)
    }
}

/// Print a formatted event to stdout
fn print_event(event: &AgentEvent, turn_context: usize) {
    let time = event.timestamp.with_timezone(&Local).format("%H:%M:%S");

    match &event.payload {
        EventPayload::User(payload) => {
            // Skip empty user messages
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
            // Skip empty reasoning blocks
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
            // Success results are not shown (too noisy)
        }
        EventPayload::Message(payload) => {
            // Skip empty messages
            if payload.text.trim().is_empty() {
                return;
            }
            let text = truncate(&payload.text, 100);
            println!("{} {} {}", time.dimmed(), "💬 Msg:".cyan(), text);
        }
        EventPayload::TokenUsage(_) => {
            // Skip token usage (sidecar info)
        }
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

/// Truncate text to max length with ellipsis
fn truncate(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        // Find the nearest char boundary at or before max_len
        let mut boundary = max_len;
        while boundary > 0 && !text.is_char_boundary(boundary) {
            boundary -= 1;
        }
        format!("{}...", &text[..boundary])
    }
}
