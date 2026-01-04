use crate::Result;
use agtrace_types::SpawnContext;
use std::io::{BufRead, BufReader};
use std::path::Path;

use super::parser::normalize_codex_session;
use super::schema::{CodexRecord, EventMsgPayload};

/// Parse Codex JSONL file and normalize to AgentEvent
pub fn normalize_codex_file(path: &Path) -> Result<Vec<agtrace_types::AgentEvent>> {
    let text = std::fs::read_to_string(path)?;

    let mut records: Vec<CodexRecord> = Vec::new();
    let mut session_id_from_meta: Option<String> = None;
    let mut subagent_type: Option<String> = None;

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let record: CodexRecord = serde_json::from_str(line)?;

        // Extract session_id and subagent_type from session_meta record
        if let CodexRecord::SessionMeta(ref meta) = record {
            session_id_from_meta = Some(meta.payload.id.clone());
            // Extract subagent information from source field
            if let super::schema::SessionSource::Subagent { subagent } = &meta.payload.source {
                subagent_type = Some(subagent.clone());
            }
        }

        records.push(record);
    }

    // session_id should be extracted from file content, fallback to "unknown-session"
    let session_id = session_id_from_meta.unwrap_or_else(|| "unknown-session".to_string());

    Ok(normalize_codex_session(records, &session_id, subagent_type))
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

/// Spawn event extracted from a CLI session (e.g., entered_review_mode)
#[derive(Debug, Clone)]
pub struct SpawnEvent {
    pub timestamp: String,
    pub subagent_type: String,
    pub spawn_context: SpawnContext,
}

#[derive(Debug)]
pub struct CodexHeader {
    pub session_id: Option<String>,
    pub cwd: Option<String>,
    pub timestamp: Option<String>,
    pub snippet: Option<String>,
    pub subagent_type: Option<String>,
    pub parent_session_id: Option<String>,
    /// Pre-computed spawn context for subagent sessions (set during discovery correlation)
    pub spawned_by: Option<SpawnContext>,
}

/// Extract header information from Codex file (for scanning)
pub fn extract_codex_header(path: &Path) -> Result<CodexHeader> {
    let file = std::fs::File::open(path)?;
    let reader = BufReader::new(file);

    let mut session_id = None;
    let mut cwd = None;
    let mut timestamp = None;
    let mut snippet = None;
    let mut subagent_type = None;
    let parent_session_id = None; // Not mutated (future use for Codex parent tracking)

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
                    // Extract subagent information from source field
                    if subagent_type.is_none()
                        && let super::schema::SessionSource::Subagent { subagent } =
                            &meta.payload.source
                    {
                        subagent_type = Some(subagent.clone());
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
                    if snippet.is_none()
                        && let super::schema::EventMsgPayload::UserMessage(msg) = &event.payload
                    {
                        snippet = Some(agtrace_types::truncate(&msg.message, 200));
                    }
                }
                CodexRecord::ResponseItem(response) => {
                    if timestamp.is_none() {
                        timestamp = Some(response.timestamp.clone());
                    }
                    if snippet.is_none()
                        && let super::schema::ResponseItemPayload::Message(msg) = &response.payload
                        && msg.role == "user"
                    {
                        let text = msg.content.iter().find_map(|c| match c {
                            super::schema::MessageContent::InputText { text } => {
                                Some(agtrace_types::truncate(text, 200))
                            }
                            super::schema::MessageContent::OutputText { text } => {
                                Some(agtrace_types::truncate(text, 200))
                            }
                            _ => None,
                        });
                        if let Some(t) = &text
                            && !t.contains("<environment_context>")
                        {
                            snippet = text;
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

    Ok(CodexHeader {
        session_id,
        cwd,
        timestamp,
        snippet,
        subagent_type,
        parent_session_id,
        spawned_by: None, // Set during discovery correlation
    })
}

/// Extract spawn events from a CLI session file with turn/step context
/// Used to correlate subagent sessions back to their parent turns
pub fn extract_spawn_events(path: &Path) -> Result<Vec<SpawnEvent>> {
    let text = std::fs::read_to_string(path)?;
    let mut spawn_events = Vec::new();

    // Track turn/step indices
    // A new turn starts with TurnContext or UserMessage
    let mut current_turn: usize = 0;
    let mut current_step: usize = 0;
    let mut in_turn = false;

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let record: CodexRecord = match serde_json::from_str(line) {
            Ok(r) => r,
            Err(_) => continue,
        };

        match &record {
            CodexRecord::TurnContext(_) => {
                // New turn starts
                if in_turn {
                    current_turn += 1;
                }
                current_step = 0;
                in_turn = true;
            }
            CodexRecord::EventMsg(event) => {
                match &event.payload {
                    EventMsgPayload::UserMessage(_) => {
                        // User message also starts a new turn (if no TurnContext)
                        if in_turn {
                            current_turn += 1;
                            current_step = 0;
                        }
                        in_turn = true;
                    }
                    EventMsgPayload::EnteredReviewMode(_) => {
                        // Found a spawn event!
                        spawn_events.push(SpawnEvent {
                            timestamp: event.timestamp.clone(),
                            subagent_type: "review".to_string(),
                            spawn_context: SpawnContext {
                                turn_index: current_turn,
                                step_index: current_step,
                            },
                        });
                        current_step += 1;
                    }
                    _ => {
                        // Other events increment step within current turn
                        if in_turn {
                            current_step += 1;
                        }
                    }
                }
            }
            CodexRecord::ResponseItem(_) => {
                // Response items are part of current step
                if in_turn {
                    current_step += 1;
                }
            }
            _ => {}
        }
    }

    Ok(spawn_events)
}

/// Check if a Codex session file is empty or incomplete
pub fn is_empty_codex_session(path: &Path) -> bool {
    let Ok(file) = std::fs::File::open(path) else {
        return true;
    };
    let reader = BufReader::new(file);

    let mut line_count = 0;
    let mut has_event = false;

    for line in reader.lines().take(20).flatten() {
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

    line_count <= 2 && !has_event
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_subagent_header() {
        // Create a temporary file with subagent session_meta
        let tmpfile = std::env::temp_dir().join("test_subagent.jsonl");
        std::fs::write(&tmpfile, r#"{"timestamp":"2025-01-01T00:00:00Z","type":"session_meta","payload":{"id":"test-id","timestamp":"2025-01-01T00:00:00Z","cwd":"/test","originator":"test","cli_version":"1.0.0","source":{"subagent":"review"}}}
"#).unwrap();

        let header = extract_codex_header(&tmpfile).unwrap();

        assert_eq!(header.session_id, Some("test-id".to_string()));
        assert_eq!(header.subagent_type, Some("review".to_string()));
        assert!(header.parent_session_id.is_none());

        std::fs::remove_file(&tmpfile).unwrap();
    }
}
