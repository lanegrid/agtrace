use crate::model::AgentEventV1;
use anyhow::{Context, Result};
use std::io::{BufRead, BufReader};
use std::path::Path;

use super::mapper::normalize_claude_stream;
use super::schema::ClaudeRecord;

/// Parse Claude Code JSONL file and normalize to AgentEventV1
pub fn normalize_claude_file(
    path: &Path,
    project_root_override: Option<&str>,
) -> Result<Vec<AgentEventV1>> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read Claude file: {}", path.display()))?;

    let mut records: Vec<ClaudeRecord> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let record: ClaudeRecord = serde_json::from_str(line)
            .with_context(|| format!("Failed to parse JSON line: {}", line))?;
        records.push(record);
    }

    Ok(normalize_claude_stream(records, project_root_override))
}

/// Extract cwd from a Claude session file by reading the first few lines
pub fn extract_cwd_from_claude_file(path: &Path) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    let reader = BufReader::new(file);

    for line in reader.lines().take(10).flatten() {
        if let Ok(record) = serde_json::from_str::<ClaudeRecord>(&line) {
            match record {
                ClaudeRecord::User(user) => {
                    if let Some(cwd) = user.cwd {
                        return Some(cwd);
                    }
                }
                ClaudeRecord::Assistant(asst) => {
                    if let Some(cwd) = asst.cwd {
                        return Some(cwd);
                    }
                }
                _ => continue,
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
    let file = std::fs::File::open(path)
        .with_context(|| format!("Failed to open file: {}", path.display()))?;
    let reader = BufReader::new(file);

    let mut session_id = None;
    let mut cwd = None;
    let mut timestamp = None;
    let mut snippet = None;

    for line in reader.lines().take(20).flatten() {
        if let Ok(record) = serde_json::from_str::<ClaudeRecord>(&line) {
            match &record {
                ClaudeRecord::User(user) => {
                    if session_id.is_none() {
                        session_id = Some(user.session_id.clone());
                    }
                    if cwd.is_none() {
                        cwd = user.cwd.clone();
                    }
                    if timestamp.is_none() {
                        timestamp = Some(user.timestamp.clone());
                    }
                    if snippet.is_none() && !user.is_sidechain {
                        snippet = user.message.content.iter()
                            .find_map(|c| match c {
                                super::schema::UserContent::Text { text } => Some(text.clone()),
                                _ => None,
                            });
                    }
                }
                ClaudeRecord::Assistant(asst) => {
                    if session_id.is_none() {
                        session_id = Some(asst.session_id.clone());
                    }
                    if cwd.is_none() {
                        cwd = asst.cwd.clone();
                    }
                    if timestamp.is_none() {
                        timestamp = Some(asst.timestamp.clone());
                    }
                }
                _ => {}
            }

            if session_id.is_some() && cwd.is_some() && timestamp.is_some() && snippet.is_some() {
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
