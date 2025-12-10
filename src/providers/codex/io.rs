use crate::model::AgentEventV1;
use anyhow::{Context, Result};
use std::io::{BufRead, BufReader};
use std::path::Path;

use super::mapper::normalize_codex_stream;
use super::schema::CodexRecord;

/// Parse Codex JSONL file and normalize to AgentEventV1
pub fn normalize_codex_file(
    path: &Path,
    fallback_session_id: &str,
    project_root_override: Option<&str>,
) -> Result<Vec<AgentEventV1>> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read Codex file: {}", path.display()))?;

    let mut records: Vec<CodexRecord> = Vec::new();
    let mut session_id_from_meta: Option<String> = None;

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let record: CodexRecord = serde_json::from_str(line)
            .with_context(|| format!("Failed to parse JSON line: {}", line))?;

        // Extract session_id from session_meta record (Spec 2.5.5)
        if let CodexRecord::SessionMeta(ref meta) = record {
            session_id_from_meta = Some(meta.payload.id.clone());
        }

        records.push(record);
    }

    // Use session_meta.payload.id if available, otherwise fallback to filename-based ID
    let session_id = session_id_from_meta
        .as_deref()
        .unwrap_or(fallback_session_id);

    Ok(normalize_codex_stream(
        records,
        session_id,
        project_root_override,
    ))
}

/// Extract cwd from a Codex session file by reading the first few records
pub fn extract_cwd_from_codex_file(path: &Path) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    let reader = BufReader::new(file);

    for line in reader.lines().take(10).flatten() {
        if let Ok(record) = serde_json::from_str::<CodexRecord>(&line) {
            match record {
                CodexRecord::SessionMeta(meta) => {
                    return Some(meta.payload.cwd.clone());
                }
                CodexRecord::TurnContext(turn) => {
                    return Some(turn.payload.cwd.clone());
                }
                _ => continue,
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
        if let Ok(record) = serde_json::from_str::<CodexRecord>(&line) {
            match &record {
                CodexRecord::SessionMeta(meta) => {
                    if session_id.is_none() {
                        session_id = Some(meta.payload.id.clone());
                    }
                    if cwd.is_none() {
                        cwd = Some(meta.payload.cwd.clone());
                    }
                    if timestamp.is_none() {
                        timestamp = Some(meta.timestamp.clone());
                    }
                }
                CodexRecord::TurnContext(turn) => {
                    if cwd.is_none() {
                        cwd = Some(turn.payload.cwd.clone());
                    }
                    if timestamp.is_none() {
                        timestamp = Some(turn.timestamp.clone());
                    }
                }
                CodexRecord::EventMsg(event) => {
                    if timestamp.is_none() {
                        timestamp = Some(event.timestamp.clone());
                    }
                    if snippet.is_none() {
                        if let super::schema::EventMsgPayload::UserMessage(msg) = &event.payload {
                            snippet = Some(msg.message.clone());
                        }
                    }
                }
                CodexRecord::ResponseItem(response) => {
                    if timestamp.is_none() {
                        timestamp = Some(response.timestamp.clone());
                    }
                    if snippet.is_none() {
                        if let super::schema::ResponseItemPayload::Message(msg) = &response.payload {
                            if msg.role == "user" {
                                let text = msg.content.iter()
                                    .find_map(|c| match c {
                                        super::schema::MessageContent::InputText { text } => Some(text.clone()),
                                        super::schema::MessageContent::OutputText { text } => Some(text.clone()),
                                        _ => None,
                                    });
                                if let Some(t) = &text {
                                    if !t.contains("<environment_context>") {
                                        snippet = text;
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }

            if session_id.is_some() && cwd.is_some() && timestamp.is_some() && snippet.is_some() {
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
            if let Ok(record) = serde_json::from_str::<CodexRecord>(&line) {
                match record {
                    CodexRecord::SessionMeta(_) | CodexRecord::TurnContext(_) => {
                        has_event = true;
                        break;
                    }
                    CodexRecord::EventMsg(_) | CodexRecord::ResponseItem(_) => {
                        has_event = true;
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    line_count <= 2 && !has_event
}
