//! Legacy index-scanning helpers (header / snippet / review-mode spawn scan).
//!
//! Used only by `CodexDiscovery` (index); replaced by `Provider::discover` + headers in the
//! workspace/index rework. Records are read as untyped JSON so the decoder schema can evolve
//! independently.

use crate::Result;
use agtrace_types::SpawnContext;
use std::io::{BufRead, BufReader};
use std::path::Path;

use serde_json::Value;

/// Parse a Codex JSONL file and normalize to AgentEvent (lenient per line).
pub fn normalize_codex_file(path: &Path) -> Result<Vec<agtrace_types::AgentEvent>> {
    let (_, events, _) = crate::provider::decode_file(
        &super::CodexProvider,
        path,
        crate::provider::DecodeOptions::default(),
    )?;
    Ok(events)
}

/// Extract cwd from a Codex session file by reading the first few records
pub fn extract_cwd_from_codex_file(path: &Path) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    let reader = BufReader::new(file);

    for line in reader.lines().take(10).flatten() {
        let Ok(record) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if matches!(record_type(&record), "session_meta" | "turn_context")
            && let Some(cwd) = payload_str(&record, "cwd")
        {
            return Some(cwd.to_string());
        }
    }
    None
}

fn record_type(record: &Value) -> &str {
    record.get("type").and_then(Value::as_str).unwrap_or("")
}

fn payload_type(record: &Value) -> &str {
    record
        .get("payload")
        .and_then(|p| p.get("type"))
        .and_then(Value::as_str)
        .unwrap_or("")
}

fn payload_str<'a>(record: &'a Value, key: &str) -> Option<&'a str> {
    record
        .get("payload")
        .and_then(|p| p.get(key))
        .and_then(Value::as_str)
}

fn record_timestamp(record: &Value) -> Option<String> {
    record
        .get("timestamp")
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// Subagent label from a `session_meta.source` value, if the session is a child.
///
/// - `{"subagent":"review"}` (legacy) → `"review"`
/// - `{"subagent":{"thread_spawn":{...}}}` → `agent_role`, else `"thread_spawn"`
fn source_subagent_label(source: &Value) -> Option<String> {
    let sub = source.get("subagent")?;
    if let Some(s) = sub.as_str() {
        return Some(s.to_string());
    }
    sub.get("thread_spawn")
        .and_then(|t| t.get("agent_role"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| sub.as_object().and_then(|o| o.keys().next().cloned()))
}

/// Text of a user prompt in a `response_item.message`, skipping injected context.
fn user_prompt_text(payload: &Value) -> Option<String> {
    if payload.get("role").and_then(Value::as_str) != Some("user") {
        return None;
    }
    // Paginated rollouts classify injected context; only `user.text` is a prompt.
    if let Some(kinds) = payload
        .get("internal_chat_message_metadata_passthrough")
        .and_then(|m| m.get("content_item_kinds"))
        .and_then(Value::as_array)
        && !kinds.iter().any(|k| k.as_str() == Some("user.text"))
    {
        return None;
    }
    let text = payload
        .get("content")
        .and_then(Value::as_array)?
        .iter()
        .find_map(|c| c.get("text").and_then(Value::as_str))?;
    if text.contains("<environment_context>") {
        return None;
    }
    Some(agtrace_types::truncate(text, 200))
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
        let Ok(record) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let payload = record.get("payload").cloned().unwrap_or(Value::Null);
        match record_type(&record) {
            "session_meta" => {
                if session_id.is_none() {
                    session_id = payload_str(&record, "id").map(str::to_string);
                }
                if cwd.is_none() {
                    cwd = payload_str(&record, "cwd").map(str::to_string);
                }
                if timestamp.is_none() {
                    timestamp = record_timestamp(&record);
                }
                if subagent_type.is_none() {
                    subagent_type = payload.get("source").and_then(source_subagent_label);
                }
            }
            "turn_context" => {
                if cwd.is_none() {
                    cwd = payload_str(&record, "cwd").map(str::to_string);
                }
                if timestamp.is_none() {
                    timestamp = record_timestamp(&record);
                }
            }
            "event_msg" => {
                if timestamp.is_none() {
                    timestamp = record_timestamp(&record);
                }
                if snippet.is_none() && payload_type(&record) == "user_message" {
                    snippet =
                        payload_str(&record, "message").map(|m| agtrace_types::truncate(m, 200));
                }
            }
            "response_item" => {
                if timestamp.is_none() {
                    timestamp = record_timestamp(&record);
                }
                if snippet.is_none() && payload_type(&record) == "message" {
                    snippet = user_prompt_text(&payload);
                }
            }
            _ => {}
        }

        if session_id.is_some() && cwd.is_some() && timestamp.is_some() && snippet.is_some() {
            break;
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

        let record: Value = match serde_json::from_str(line) {
            Ok(r) => r,
            Err(_) => continue,
        };

        match record_type(&record) {
            "turn_context" => {
                // New turn starts
                if in_turn {
                    current_turn += 1;
                }
                current_step = 0;
                in_turn = true;
            }
            "event_msg" => match payload_type(&record) {
                "user_message" => {
                    // User message also starts a new turn (if no TurnContext)
                    if in_turn {
                        current_turn += 1;
                        current_step = 0;
                    }
                    in_turn = true;
                }
                "entered_review_mode" => {
                    spawn_events.push(SpawnEvent {
                        timestamp: record_timestamp(&record).unwrap_or_default(),
                        subagent_type: "review".to_string(),
                        spawn_context: SpawnContext {
                            turn_index: current_turn,
                            step_index: current_step,
                        },
                    });
                    current_step += 1;
                }
                _ => {
                    if in_turn {
                        current_step += 1;
                    }
                }
            },
            // Response items are part of current step
            "response_item" if in_turn => {
                current_step += 1;
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
        if let Ok(record) = serde_json::from_str::<Value>(&line)
            && matches!(
                record_type(&record),
                "session_meta" | "turn_context" | "event_msg" | "response_item"
            )
        {
            has_event = true;
            break;
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
