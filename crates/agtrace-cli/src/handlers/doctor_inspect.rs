use crate::args::{InspectFormat, OutputFormat, ViewModeArgs};
use agtrace_sdk::SystemClient;
use agtrace_sdk::types::InspectContentType;
use anyhow::Result;
use std::path::Path;

pub fn handle(
    file_path: String,
    lines: usize,
    inspect_format: InspectFormat,
    output_format: OutputFormat,
    view_mode: &ViewModeArgs,
) -> Result<()> {
    use crate::presentation::presenters;
    use crate::presentation::{ConsoleRenderer, Renderer};

    let json_format = matches!(inspect_format, InspectFormat::Json);
    let path = Path::new(&file_path);
    let result = SystemClient::inspect_file(path, lines, json_format)?;

    // Convert lines to strings for presentation
    let formatted_lines: Vec<(usize, String)> = result
        .lines
        .into_iter()
        .map(|line| {
            let content = match line.content {
                InspectContentType::Raw(s) => s,
                InspectContentType::Json(v) => {
                    serde_json::to_string_pretty(&v).unwrap_or_else(|_| v.to_string())
                }
            };
            (line.number, content)
        })
        .collect();

    let view_model = presenters::present_inspect_result(
        result.file_path,
        result.total_lines,
        result.shown_lines,
        formatted_lines,
    );

    let presentation_format = crate::presentation::OutputFormat::from(output_format);
    let renderer = ConsoleRenderer::new(presentation_format, view_mode.resolve());
    renderer.render(view_model)?;

    Ok(())
}
