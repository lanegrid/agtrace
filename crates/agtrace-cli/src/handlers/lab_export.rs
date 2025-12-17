use crate::session_loader::{LoadOptions, SessionLoader};
use crate::types::{ExportFormat, ExportStrategy as CliExportStrategy};
use crate::ui::TraceView;
use agtrace_engine::export::{self, ExportStrategy};
use agtrace_index::Database;
use agtrace_types::v2::{AgentEvent, EventPayload};
use anyhow::{Context, Result};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub fn handle(
    db: &Database,
    session_id: String,
    output: Option<PathBuf>,
    format: ExportFormat,
    strategy: CliExportStrategy,
    view: &dyn TraceView,
) -> Result<()> {
    let loader = SessionLoader::new(db);
    let options = LoadOptions::default();
    let events_v2 = loader.load_events_v2(&session_id, &options)?;
    let resolved_id = session_id.clone();

    let export_strategy: ExportStrategy = strategy
        .to_string()
        .parse()
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let processed_events = export::transform(&events_v2, export_strategy);

    let output_path = output.unwrap_or_else(|| {
        PathBuf::from(format!(
            "session_{}.{}",
            &resolved_id[..8],
            match format {
                ExportFormat::Jsonl => "jsonl",
                ExportFormat::Text => "txt",
            }
        ))
    });

    match format {
        ExportFormat::Jsonl => write_jsonl(&output_path, &processed_events)?,
        ExportFormat::Text => write_text(&output_path, &processed_events)?,
    }

    view.render_lab_export(processed_events.len(), &output_path)?;

    Ok(())
}

fn write_jsonl(path: &Path, events: &[AgentEvent]) -> Result<()> {
    let mut file = fs::File::create(path)
        .with_context(|| format!("Failed to create file: {}", path.display()))?;

    for event in events {
        let json = serde_json::to_string(event)?;
        writeln!(file, "{}", json)?;
    }

    Ok(())
}

fn write_text(path: &Path, events: &[AgentEvent]) -> Result<()> {
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
                writeln!(file, "Tool: {}", p.name)?;
                writeln!(file, "Args: {}", p.arguments)?;
            }
            EventPayload::ToolResult(p) => {
                writeln!(file, "{}", p.output)?;
            }
            EventPayload::TokenUsage(p) => {
                writeln!(
                    file,
                    "Tokens: in={}, out={}",
                    p.input_tokens, p.output_tokens
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
        }

        writeln!(file)?;
    }

    Ok(())
}
