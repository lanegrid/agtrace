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

#[derive(Debug)]
pub struct ClaudeHeader {
    pub session_id: Option<String>,
    pub cwd: Option<String>,
    pub timestamp: Option<String>,
    pub snippet: Option<String>,
}

/// Extract header information from Claude file (for scanning)
pub fn extract_claude_header(path: &Path) -> Result<ClaudeHeader> {
    use anyhow::Context;

    let file = std::fs::File::open(path)
        .with_context(|| format!("Failed to open file: {}", path.display()))?;
    let reader = BufReader::new(file);

    let mut session_id = None;
    let mut cwd = None;
    let mut timestamp = None;
    let mut snippet = None;

    for line in reader.lines().take(20).flatten() {
        if let Ok(json) = serde_json::from_str::<Value>(&line) {
            if session_id.is_none() {
                session_id = json.get("sessionId").and_then(|v| v.as_str()).map(String::from);
            }

            if cwd.is_none() {
                cwd = json.get("cwd").and_then(|v| v.as_str()).map(String::from);
            }

            if timestamp.is_none() {
                timestamp = json.get("timestamp").and_then(|v| v.as_str()).map(String::from)
                    .or_else(|| json.get("ts").and_then(|v| v.as_str()).map(String::from));
            }

            if snippet.is_none() {
                if json.get("type").and_then(|v| v.as_str()) == Some("message") {
                    if json.get("message").and_then(|m| m.get("role")).and_then(|r| r.as_str()) == Some("user") {
                        snippet = json.get("message")
                            .and_then(|m| m.get("content"))
                            .and_then(|c| {
                                if let Some(s) = c.as_str() {
                                    Some(s.to_string())
                                } else if let Some(arr) = c.as_array() {
                                    arr.iter()
                                        .find_map(|item| item.get("text").and_then(|t| t.as_str()))
                                        .map(String::from)
                                } else {
                                    None
                                }
                            });
                    }
                }
            }

            if session_id.is_some() && cwd.is_some() && timestamp.is_some() {
                break;
            }
        }
    }

    Ok(ClaudeHeader {
        session_id,
        cwd,
        timestamp,
        snippet,
    })
}
