use crate::Result;
use std::io::{BufRead, BufReader};
use std::path::Path;

use super::parser::normalize_claude_session;
use super::schema::ClaudeRecord;

/// Parse Claude Code JSONL file and normalize to AgentEvent
pub fn normalize_claude_file(path: &Path) -> Result<Vec<agtrace_types::AgentEvent>> {
    let text = std::fs::read_to_string(path)?;

    let mut records: Vec<ClaudeRecord> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let record: ClaudeRecord = serde_json::from_str(line)?;
        records.push(record);
    }

    Ok(normalize_claude_session(records))
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
    pub is_sidechain: bool,
}

/// Extract header information from Claude file (for scanning)
pub fn extract_claude_header(path: &Path) -> Result<ClaudeHeader> {
    let file = std::fs::File::open(path)?;
    let reader = BufReader::new(file);

    let mut session_id = None;
    let mut cwd = None;
    let mut timestamp = None;
    let mut snippet = None;
    let mut is_sidechain = false;
    let mut meta_message_ids = std::collections::HashSet::new();

    for line in reader.lines().take(200).flatten() {
        if let Ok(record) = serde_json::from_str::<ClaudeRecord>(&line) {
            match &record {
                ClaudeRecord::FileHistorySnapshot(_) => {
                    // File history snapshots mark the end of a meta message chain
                    meta_message_ids.clear();
                }
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

                    // Track meta message IDs and their descendants
                    if user.is_meta {
                        meta_message_ids.insert(user.uuid.clone());
                    }

                    // Check if this message's parent is a meta message (or descendant)
                    let parent_is_meta = user
                        .parent_uuid
                        .as_ref()
                        .map(|p| meta_message_ids.contains(p))
                        .unwrap_or(false);

                    // If parent is meta, this message is also considered meta-related
                    if parent_is_meta {
                        meta_message_ids.insert(user.uuid.clone());
                    }

                    // Extract snippet from first non-sidechain, non-meta user message
                    // Also skip messages whose parent is meta
                    if snippet.is_none() && !user.is_sidechain && !user.is_meta && !parent_is_meta {
                        snippet = user.message.content.iter().find_map(|c| match c {
                            super::schema::UserContent::Text { text } => Some(text.clone()),
                            _ => None,
                        });
                    }
                    is_sidechain = user.is_sidechain;
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
        is_sidechain,
    })
}
