use crate::presentation::renderers::TraceView;
use crate::services::writer;
use crate::types::{ExportFormat, ExportStrategy as CliExportStrategy};
use agtrace_engine::export::{self, ExportStrategy};
use agtrace_index::Database;
use agtrace_runtime::{LoadOptions, SessionRepository};
use anyhow::Result;
use std::path::PathBuf;

pub fn handle(
    db: &Database,
    session_id: String,
    output: Option<PathBuf>,
    format: ExportFormat,
    strategy: CliExportStrategy,
    view: &dyn TraceView,
) -> Result<()> {
    let loader = SessionRepository::new(db);
    let options = LoadOptions::default();
    let events = loader.load_events(&session_id, &options)?;
    let resolved_id = session_id.clone();

    let export_strategy: ExportStrategy = strategy
        .to_string()
        .parse()
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let processed_events = export::transform(&events, export_strategy);

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
        ExportFormat::Jsonl => writer::write_jsonl(&output_path, &processed_events)?,
        ExportFormat::Text => writer::write_text(&output_path, &processed_events)?,
    }

    view.render_lab_export(processed_events.len(), &output_path)?;

    Ok(())
}
