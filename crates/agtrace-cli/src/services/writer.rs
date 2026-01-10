use agtrace_sdk::types::{AgentEvent, EventPayload};
use anyhow::{Context, Result};
use std::fs;
use std::io::Write;
use std::path::Path;

pub fn write_jsonl(path: &Path, events: &[AgentEvent]) -> Result<()> {
    let mut file = fs::File::create(path)
        .with_context(|| format!("Failed to create file: {}", path.display()))?;

    for event in events {
        let json = serde_json::to_string(event)?;
        writeln!(file, "{}", json)?;
    }

    Ok(())
}

pub fn write_text(path: &Path, events: &[AgentEvent]) -> Result<()> {
    let mut file = fs::File::create(path)
        .with_context(|| format!("Failed to create file: {}", path.display()))?;

    for event in events {
        let ts_str = event.timestamp.to_rfc3339();
        let event_type = match &event.payload {
            EventPayload::User(_) => "User",
            EventPayload::Message(_) => "Message",
            EventPayload::Reasoning(_) => "Reasoning",
            EventPayload::ToolCall(_) => "ToolCall",
            EventPayload::ToolResult(_) => "ToolResult",
            EventPayload::TokenUsage(_) => "TokenUsage",
            EventPayload::Notification(_) => "Notification",
            EventPayload::SlashCommand(_) => "SlashCommand",
        };

        writeln!(file, "[{}] {}", ts_str, event_type)?;

        match &event.payload {
            EventPayload::User(p) => {
                writeln!(file, "{}", p.text)?;
            }
            EventPayload::Message(p) => {
                writeln!(file, "{}", p.text)?;
            }
            EventPayload::Reasoning(p) => {
                writeln!(file, "{}", p.text)?;
            }
            EventPayload::ToolCall(p) => {
                writeln!(file, "Tool: {}", p.name())?;
                let arguments = serde_json::to_value(p)
                    .ok()
                    .and_then(|v| v.get("arguments").cloned())
                    .unwrap_or(serde_json::Value::Null);
                writeln!(file, "Args: {}", arguments)?;
            }
            EventPayload::ToolResult(p) => {
                writeln!(file, "{}", p.output)?;
            }
            EventPayload::TokenUsage(p) => {
                writeln!(
                    file,
                    "Tokens: in={}, out={}",
                    p.input.total(),
                    p.output.total()
                )?;
            }
            EventPayload::Notification(p) => {
                writeln!(
                    file,
                    "[{}] {}",
                    p.level.as_deref().unwrap_or("info"),
                    p.text
                )?;
            }
            EventPayload::SlashCommand(p) => {
                if let Some(args) = &p.args {
                    writeln!(file, "{} {}", p.name, args)?;
                } else {
                    writeln!(file, "{}", p.name)?;
                }
            }
        }

        writeln!(file)?;
    }

    Ok(())
}
