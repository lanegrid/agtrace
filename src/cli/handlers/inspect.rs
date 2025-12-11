use anyhow::{Context, Result};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub fn handle(file_path: String, lines: usize, format: String) -> Result<()> {
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

    println!("File: {}", file_path);
    println!("Lines: 1-{} (total: {} lines)", lines.min(total_lines), total_lines);
    println!("{}", "─".repeat(40));

    match format.as_str() {
        "json" => display_json(reader, lines)?,
        _ => display_raw(reader, lines)?,
    }

    println!("{}", "─".repeat(40));

    Ok(())
}

fn display_raw(reader: BufReader<File>, max_lines: usize) -> Result<()> {
    for (idx, line) in reader.lines().take(max_lines).enumerate() {
        let line = line?;
        println!("{:>6}  {}", idx + 1, line);
    }
    Ok(())
}

fn display_json(reader: BufReader<File>, max_lines: usize) -> Result<()> {
    for (idx, line) in reader.lines().take(max_lines).enumerate() {
        let line = line?;

        // Try to parse and pretty-print JSON
        match serde_json::from_str::<serde_json::Value>(&line) {
            Ok(json) => {
                println!("{:>6}  {}", idx + 1, serde_json::to_string_pretty(&json)?);
            }
            Err(_) => {
                // If not valid JSON, display as-is
                println!("{:>6}  {}", idx + 1, line);
            }
        }
    }
    Ok(())
}
