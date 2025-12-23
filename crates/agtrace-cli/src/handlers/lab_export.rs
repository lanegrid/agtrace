use crate::args::{ExportFormat, ExportStrategy as CliExportStrategy};
use crate::presentation::v1::renderers::TraceView;
use crate::services::writer;
use agtrace_engine::export::ExportStrategy;
use agtrace_runtime::AgTrace;
use anyhow::Result;
use std::path::PathBuf;

pub fn handle(
    workspace: &AgTrace,
    session_id: String,
    output: Option<PathBuf>,
    format: ExportFormat,
    strategy: CliExportStrategy,
    view: &dyn TraceView,
) -> Result<()> {
    let export_strategy: ExportStrategy = strategy
        .to_string()
        .parse()
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let session = workspace.sessions().find(&session_id)?;
    let processed_events = session.export(export_strategy)?;

    let output_path = output.unwrap_or_else(|| {
        PathBuf::from(format!(
            "session_{}.{}",
            &session_id[..8.min(session_id.len())],
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
