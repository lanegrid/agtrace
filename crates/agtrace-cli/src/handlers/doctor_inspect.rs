use crate::types::InspectFormat;
use crate::presentation::renderers::models::{InspectContent, InspectDisplay, InspectLine};
use crate::presentation::renderers::TraceView;
use anyhow::{Context, Result};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub fn handle(
    file_path: String,
    lines: usize,
    format: InspectFormat,
    view: &dyn TraceView,
) -> Result<()> {
    let path = Path::new(&file_path);

    if !path.exists() {
        anyhow::bail!("File not found: {}", file_path);
    }

    let file = File::open(path).with_context(|| format!("Failed to open file: {}", file_path))?;
    let reader = BufReader::new(file);

    // Count total lines
    let total_lines = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", file_path))?
        .lines()
        .count();

    let mut rendered_lines = Vec::new();
    for (idx, line) in reader.lines().take(lines).enumerate() {
        let line = line?;
        let content = match format {
            InspectFormat::Raw => InspectContent::Raw(line.clone()),
            InspectFormat::Json => match serde_json::from_str::<serde_json::Value>(&line) {
                Ok(json) => InspectContent::Json(json),
                Err(_) => InspectContent::Raw(line.clone()),
            },
        };
        rendered_lines.push(InspectLine {
            number: idx + 1,
            content,
        });
    }

    let display = InspectDisplay {
        file_path,
        total_lines,
        shown_lines: rendered_lines.len(),
        lines: rendered_lines,
    };

    view.render_inspect(&display)?;

    Ok(())
}
