use crate::model::AgentEventV1;
use anyhow::{Context, Result};
use serde_json::Value;
use std::io::{BufRead, BufReader};
use std::path::Path;

use super::mapper::normalize_claude_stream;

/// Parse Claude Code JSONL file and normalize to AgentEventV1
pub fn normalize_claude_file(
    path: &Path,
    project_root_override: Option<&str>,
) -> Result<Vec<AgentEventV1>> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read Claude file: {}", path.display()))?;

    let mut records: Vec<Value> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let v: Value = serde_json::from_str(line)
            .with_context(|| format!("Failed to parse JSON line: {}", line))?;
        records.push(v);
    }

    Ok(normalize_claude_stream(
        records.into_iter(),
        project_root_override,
    ))
}

/// Extract cwd from a Claude session file by reading the first few lines
pub fn extract_cwd_from_claude_file(path: &Path) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    let reader = BufReader::new(file);

    for line in reader.lines().take(10).flatten() {
        if let Ok(json) = serde_json::from_str::<Value>(&line) {
            if let Some(cwd) = json.get("cwd").and_then(|v| v.as_str()) {
                return Some(cwd.to_string());
            }
        }
    }
    None
}
