use crate::presentation::renderers::TraceView;
use crate::presentation::view_models::{InspectContent, InspectDisplay, InspectLine};
use crate::types::InspectFormat;
use agtrace_runtime::{AgTrace, InspectContentType};
use anyhow::Result;

pub fn handle(
    file_path: String,
    lines: usize,
    format: InspectFormat,
    view: &dyn TraceView,
) -> Result<()> {
    let json_format = matches!(format, InspectFormat::Json);
    let result = AgTrace::inspect_file(&file_path, lines, json_format)?;

    let display = InspectDisplay {
        file_path: result.file_path,
        total_lines: result.total_lines,
        shown_lines: result.shown_lines,
        lines: result
            .lines
            .into_iter()
            .map(|line| InspectLine {
                number: line.number,
                content: match line.content {
                    InspectContentType::Raw(s) => InspectContent::Raw(s),
                    InspectContentType::Json(v) => InspectContent::Json(v),
                },
            })
            .collect(),
    };

    view.render_inspect(&display)?;

    Ok(())
}
