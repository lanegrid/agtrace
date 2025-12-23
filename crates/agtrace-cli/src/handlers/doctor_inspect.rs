use crate::args::{InspectFormat, OutputFormat};
use agtrace_runtime::AgTrace;
use anyhow::Result;

pub fn handle_v2(
    file_path: String,
    lines: usize,
    inspect_format: InspectFormat,
    output_format: OutputFormat,
) -> Result<()> {
    use crate::presentation::v2::presenters;
    use crate::presentation::v2::{ConsoleRenderer, Renderer};

    let json_format = matches!(inspect_format, InspectFormat::Json);
    let result = AgTrace::inspect_file(&file_path, lines, json_format)?;

    // Convert lines to strings for presentation
    let formatted_lines: Vec<(usize, String)> = result
        .lines
        .into_iter()
        .map(|line| {
            let content = match line.content {
                agtrace_runtime::InspectContentType::Raw(s) => s,
                agtrace_runtime::InspectContentType::Json(v) => {
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

    let v2_format = crate::presentation::v2::OutputFormat::from(output_format);
    let renderer = ConsoleRenderer::new(v2_format, crate::presentation::v2::ViewMode::default());
    renderer.render(view_model)?;

    Ok(())
}
