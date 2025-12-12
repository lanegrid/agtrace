use crate::session_loader::{LoadOptions, SessionLoader};
use agtrace_engine::export::{self, ExportStrategy};
use agtrace_index::Database;
use agtrace_types::AgentEventV1;
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
    let all_events = loader.load_events(&session_id, &options)?;
    let resolved_id = session_id.clone();

    let export_strategy = ExportStrategy::from_str(&strategy)
        .ok_or_else(|| anyhow::anyhow!("Unknown strategy: {}", strategy))?;

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
