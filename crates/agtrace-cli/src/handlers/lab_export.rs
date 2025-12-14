use crate::session_loader::{LoadOptions, SessionLoader};
use agtrace_engine::export::{self, ExportStrategy};
use agtrace_index::Database;
use agtrace_types::v2::{AgentEvent, EventPayload};
use agtrace_types::{AgentEventV1, EventType, Source};
use anyhow::{Context, Result};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub fn handle(
    db: &Database,
    session_id: String,
    output: Option<PathBuf>,
    format: String,
    strategy: String,
) -> Result<()> {
    let loader = SessionLoader::new(db);
    let options = LoadOptions::default();
    // Load using v2 pipeline, then convert to v1 for export
    // TODO: Create native v2 export functions
    let events_v2 = loader.load_events_v2(&session_id, &options)?;
    let all_events = convert_v2_to_v1(&events_v2);
    let resolved_id = session_id.clone();

    let export_strategy: ExportStrategy = strategy.parse().map_err(|e| anyhow::anyhow!("{}", e))?;

    let processed_events = export::transform(&all_events, export_strategy);

    let output_path = output.unwrap_or_else(|| {
        PathBuf::from(format!(
            "session_{}.{}",
            &resolved_id[..8],
            if format == "jsonl" { "jsonl" } else { "txt" }
        ))
    });

    match format.as_str() {
        "jsonl" => write_jsonl(&output_path, &processed_events)?,
        "text" => write_text(&output_path, &processed_events)?,
        _ => anyhow::bail!("Unsupported format: {}", format),
    }

    println!(
        "Exported {} events to {}",
        processed_events.len(),
        output_path.display()
    );

    Ok(())
}

fn write_jsonl(path: &Path, events: &[AgentEventV1]) -> Result<()> {
    let mut file = fs::File::create(path)
        .with_context(|| format!("Failed to create file: {}", path.display()))?;

    for event in events {
        let json = serde_json::to_string(event)?;
        writeln!(file, "{}", json)?;
    }

    Ok(())
}

fn write_text(path: &Path, events: &[AgentEventV1]) -> Result<()> {
    let mut file = fs::File::create(path)
        .with_context(|| format!("Failed to create file: {}", path.display()))?;

    for event in events {
        writeln!(file, "[{}] {:?}", event.ts, event.event_type)?;

        if let Some(text) = &event.text {
            writeln!(file, "{}", text)?;
        }

        if let Some(tool_name) = &event.tool_name {
            writeln!(file, "Tool: {}", tool_name)?;
        }

        writeln!(file)?;
    }

    Ok(())
}

/// Convert v2 events to v1 format for compatibility with legacy export functions
fn convert_v2_to_v1(events: &[AgentEvent]) -> Vec<AgentEventV1> {
    events
        .iter()
        .filter_map(|e| {
            // Skip TokenUsage events in v1 representation
            if matches!(e.payload, EventPayload::TokenUsage(_)) {
                return None;
            }

            let event_type = match &e.payload {
                EventPayload::User(_) => EventType::UserMessage,
                EventPayload::Message(_) => EventType::AssistantMessage,
                EventPayload::ToolCall(_) => EventType::ToolCall,
                EventPayload::ToolResult(_) => EventType::ToolResult,
                EventPayload::Reasoning(_) => EventType::Reasoning,
                EventPayload::TokenUsage(_) => return None,
            };

            let text = match &e.payload {
                EventPayload::User(p) => Some(p.text.clone()),
                EventPayload::Message(p) => Some(p.text.clone()),
                EventPayload::Reasoning(p) => Some(p.text.clone()),
                EventPayload::ToolResult(p) => Some(p.output.clone()),
                EventPayload::ToolCall(p) => Some(format!("{}: {}", p.name, p.arguments)),
                _ => None,
            };

            let tool_name = match &e.payload {
                EventPayload::ToolCall(p) => Some(p.name.clone()),
                _ => None,
            };

            Some(AgentEventV1 {
                schema_version: AgentEventV1::SCHEMA_VERSION.to_string(),
                source: Source::new("unknown"),
                project_hash: String::new(),
                project_root: None,
                session_id: Some(e.trace_id.to_string()),
                event_id: Some(e.id.to_string()),
                parent_event_id: e.parent_id.map(|id| id.to_string()),
                ts: e.timestamp.to_rfc3339(),
                event_type,
                role: None,
                channel: None,
                text,
                context: None,
                policy: None,
                tool_name,
                tool_call_id: None,
                tool_status: None,
                tool_latency_ms: None,
                tool_exit_code: None,
                file_path: None,
                file_language: None,
                file_op: None,
                model: None,
                tokens_input: None,
                tokens_output: None,
                tokens_total: None,
                tokens_cached: None,
                tokens_thinking: None,
                tokens_tool: None,
                agent_id: None,
                raw: serde_json::Value::Null,
            })
        })
        .collect()
}
