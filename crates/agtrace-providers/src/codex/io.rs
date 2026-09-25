//! Codex file helpers outside the decoder: whole-file normalization and the session
//! snippet (first user prompt) for the session index. Records are read as untyped JSON so
//! the decoder schema can evolve independently.

use crate::Result;
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

/// First user prompt of a rollout (head read, ≤ 200 records), truncated to 200 chars.
///
/// Paginated rollouts start with injected context (developer / environment messages);
/// only `user.text` items count as prompts. Fork prefixes are copied parent history,
/// so the first prompt found may belong to the parent (acceptable for a snippet).
pub fn read_snippet(path: &Path) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    for line in BufReader::new(file).lines().take(200).map_while(|l| l.ok()) {
        let Ok(record) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let snippet = match (record_type(&record), payload_type(&record)) {
            ("event_msg", "user_message") => {
                payload_str(&record, "message").map(|m| agtrace_types::truncate(m, 200))
            }
            ("response_item", "message") => record.get("payload").and_then(user_prompt_text),
            _ => None,
        };
        if snippet.is_some() {
            return snippet;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snippet_skips_injected_context() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rollout-x.jsonl");
        std::fs::write(
            &path,
            concat!(
                r#"{"type":"session_meta","payload":{"id":"t"}}"#, "\n",
                r#"{"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"<environment_context>x</environment_context>"}]}}"#, "\n",
                r#"{"type":"response_item","payload":{"type":"message","role":"user","internal_chat_message_metadata_passthrough":{"content_item_kinds":["user.injected"]},"content":[{"type":"input_text","text":"injected"}]}}"#, "\n",
                r#"{"type":"response_item","payload":{"type":"message","role":"user","internal_chat_message_metadata_passthrough":{"content_item_kinds":["user.text"]},"content":[{"type":"input_text","text":"Review the parser"}]}}"#, "\n",
            ),
        )
        .unwrap();
        assert_eq!(read_snippet(&path).as_deref(), Some("Review the parser"));
    }
}
