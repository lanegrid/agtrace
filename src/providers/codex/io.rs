use crate::model::AgentEventV1;
use anyhow::{Context, Result};
use serde_json::Value;
use std::io::{BufRead, BufReader};
use std::path::Path;

use super::mapper::normalize_codex_stream;

/// Parse Codex JSONL file and normalize to AgentEventV1
pub fn normalize_codex_file(
    path: &Path,
    fallback_session_id: &str,
    project_root_override: Option<&str>,
) -> Result<Vec<AgentEventV1>> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read Codex file: {}", path.display()))?;

    let mut records: Vec<Value> = Vec::new();
    let mut session_id_from_meta: Option<String> = None;

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let v: Value = serde_json::from_str(line)
            .with_context(|| format!("Failed to parse JSON line: {}", line))?;

        // Extract session_id from session_meta record (Spec 2.5.5)
        if v.get("type").and_then(|t| t.as_str()) == Some("session_meta") {
            if let Some(id) = v
                .get("payload")
                .and_then(|p| p.get("id"))
                .and_then(|id| id.as_str())
            {
                session_id_from_meta = Some(id.to_string());
            }
        }

        records.push(v);
    }

    // Use session_meta.payload.id if available, otherwise fallback to filename-based ID
    let session_id = session_id_from_meta
        .as_deref()
        .unwrap_or(fallback_session_id);

    Ok(normalize_codex_stream(
        records.into_iter(),
        session_id,
        project_root_override,
    ))
}

/// Extract cwd from a Codex session file by reading the first few records
pub fn extract_cwd_from_codex_file(path: &Path) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    let reader = BufReader::new(file);

    for line in reader.lines().take(10).flatten() {
        if let Ok(json) = serde_json::from_str::<Value>(&line) {
            if let Some(payload) = json.get("payload") {
                if let Some(cwd) = payload.get("cwd").and_then(|v| v.as_str()) {
                    return Some(cwd.to_string());
                }
            }
        }
    }
    None
}

#[derive(Debug)]
pub struct CodexHeader {
    pub session_id: Option<String>,
    pub cwd: Option<String>,
    pub timestamp: Option<String>,
    pub snippet: Option<String>,
}

/// Extract header information from Codex file (for scanning)
pub fn extract_codex_header(path: &Path) -> Result<CodexHeader> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("Failed to open file: {}", path.display()))?;
    let reader = BufReader::new(file);

    let mut session_id = None;
    let mut cwd = None;
    let mut timestamp = None;
    let mut snippet = None;

    for line in reader.lines().take(20).flatten() {
        if let Ok(json) = serde_json::from_str::<Value>(&line) {
            if json.get("type").and_then(|t| t.as_str()) == Some("session_meta") {
                if let Some(payload) = json.get("payload") {
                    if session_id.is_none() {
                        session_id = payload.get("id").and_then(|v| v.as_str()).map(String::from);
                    }
                    if cwd.is_none() {
                        cwd = payload.get("cwd").and_then(|v| v.as_str()).map(String::from);
                    }
                }
            }

            if let Some(payload) = json.get("payload") {
                if cwd.is_none() {
                    cwd = payload.get("cwd").and_then(|v| v.as_str()).map(String::from);
                }
            }

            if timestamp.is_none() {
                timestamp = json.get("timestamp").and_then(|v| v.as_str()).map(String::from);
            }

            if snippet.is_none() {
                if json.get("payload").and_then(|p| p.get("type")).and_then(|t| t.as_str()) == Some("message") {
                    if json.get("payload").and_then(|p| p.get("role")).and_then(|r| r.as_str()) == Some("user") {
                        snippet = json.get("payload")
                            .and_then(|p| p.get("content"))
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

    if session_id.is_none() {
        if let Some(filename) = path.file_stem().and_then(|s| s.to_str()) {
            session_id = Some(filename.to_string());
        }
    }

    Ok(CodexHeader {
        session_id,
        cwd,
        timestamp,
        snippet,
    })
}

/// Check if a Codex session file is empty or incomplete
pub fn is_empty_codex_session(path: &Path) -> bool {
    let Ok(file) = std::fs::File::open(path) else {
        return true;
    };
    let reader = BufReader::new(file);

    let mut line_count = 0;
    let mut has_event = false;

    for line in reader.lines().take(20) {
        if let Ok(line) = line {
            line_count += 1;
            if let Ok(json) = serde_json::from_str::<Value>(&line) {
                if let Some(payload) = json.get("payload") {
                    if payload.get("cwd").is_some()
                        || payload.get("text").is_some()
                        || payload.get("content").is_some()
                    {
                        has_event = true;
                        break;
                    }
                }
            }
        }
    }

    line_count <= 2 && !has_event
}
