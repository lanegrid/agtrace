use crate::args::{ExportFormat, ExportStrategy as CliExportStrategy, OutputFormat, ViewModeArgs};
use crate::presentation::presenters;
use crate::presentation::view_models::CommandResultViewModel;
use crate::presentation::{ConsoleRenderer, Renderer};
use crate::services::writer;
use agtrace_sdk::Client;
use agtrace_sdk::types::ExportStrategy;
use anyhow::Result;
use std::path::PathBuf;

pub fn handle(
    client: &Client,
    session_id: String,
    output: Option<PathBuf>,
    format: ExportFormat,
    strategy: CliExportStrategy,
    output_format: OutputFormat,
    view_mode_args: &ViewModeArgs,
) -> Result<()> {
    let export_strategy: ExportStrategy = strategy
        .to_string()
        .parse()
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let session = client.sessions().get(&session_id)?;
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

    let vm = presenters::present_lab_export(processed_events.len(), &output_path);
    let result = CommandResultViewModel::new(vm);
    let resolved_view_mode = view_mode_args.resolve();
    let renderer = ConsoleRenderer::new(output_format.into(), resolved_view_mode);
    renderer.render(result)?;

    Ok(())
}
