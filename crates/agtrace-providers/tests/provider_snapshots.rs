use std::path::Path;

// Helper function to redact UUIDs from JSON for snapshot testing
fn redact_uuids(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, val) in map.iter_mut() {
                if key == "id" || key == "trace_id" || key == "parent_id" || key == "tool_call_id" {
                    if val.is_string() || val.is_null() {
                        *val = serde_json::Value::String("<UUID_REDACTED>".to_string());
                    }
                } else {
                    redact_uuids(val);
                }
            }
        }
        serde_json::Value::Array(arr) => {
            for val in arr.iter_mut() {
                redact_uuids(val);
            }
        }
        _ => {}
    }
}

// Snapshot tests - test provider normalization
#[test]
fn test_gemini_parse_snapshot() {
    let path = Path::new("tests/samples/gemini_session.json");

    if !path.exists() {
        eprintln!("Warning: Test file not found, skipping: {}", path.display());
        return;
    }

    let events = agtrace_providers::normalize_gemini_file_v2(path)
        .expect("Failed to parse Gemini file with v2");

    assert!(!events.is_empty(), "Expected at least one event");

    // Snapshot all events in pretty JSON format with UUIDs redacted
    let json_pretty = events
        .iter()
        .map(|e| {
            let mut value = serde_json::to_value(e).unwrap();
            redact_uuids(&mut value);
            serde_json::to_string_pretty(&value).unwrap()
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    insta::assert_snapshot!("gemini_events_sample", json_pretty);
}

#[test]
fn test_codex_parse_snapshot() {
    let path = Path::new("tests/samples/codex_session.jsonl");

    if !path.exists() {
        eprintln!("Warning: Test file not found, skipping: {}", path.display());
        return;
    }

    let events = agtrace_providers::normalize_codex_file_v2(path)
        .expect("Failed to parse Codex file with v2");

    assert!(!events.is_empty(), "Expected at least one event");

    // Snapshot all events in pretty JSON format with UUIDs redacted
    let json_pretty = events
        .iter()
        .map(|e| {
            let mut value = serde_json::to_value(e).unwrap();
            redact_uuids(&mut value);
            serde_json::to_string_pretty(&value).unwrap()
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    insta::assert_snapshot!("codex_events_sample", json_pretty);
}

#[test]
fn test_claude_parse_snapshot() {
    let path = Path::new("tests/samples/claude_session.jsonl");

    if !path.exists() {
        eprintln!("Warning: Test file not found, skipping: {}", path.display());
        return;
    }

    let events = agtrace_providers::normalize_claude_file_v2(path)
        .expect("Failed to parse Claude file with v2");

    assert!(!events.is_empty(), "Expected at least one event");

    // Snapshot all events in pretty JSON format with UUIDs redacted
    let json_pretty = events
        .iter()
        .map(|e| {
            let mut value = serde_json::to_value(e).unwrap();
            redact_uuids(&mut value);
            serde_json::to_string_pretty(&value).unwrap()
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    insta::assert_snapshot!("claude_events_sample", json_pretty);
}

#[test]
fn test_gemini_parse_raw_snapshot() {
    let path = Path::new("tests/samples/gemini_session.json");

    if !path.exists() {
        eprintln!("Warning: Test file not found, skipping: {}", path.display());
        return;
    }

    let events = agtrace_providers::normalize_gemini_file_v2(path)
        .expect("Failed to parse Gemini file with v2");

    assert!(!events.is_empty(), "Expected at least one event");

    // Snapshot only metadata field from all events in pretty JSON format with blank lines between
    let metadata_json_pretty = events
        .iter()
        .map(|e| serde_json::to_string_pretty(&e.metadata).unwrap())
        .collect::<Vec<_>>()
        .join("\n\n");
    insta::assert_snapshot!("gemini_events_raw", metadata_json_pretty);
}

#[test]
fn test_codex_parse_raw_snapshot() {
    let path = Path::new("tests/samples/codex_session.jsonl");

    if !path.exists() {
        eprintln!("Warning: Test file not found, skipping: {}", path.display());
        return;
    }

    let events = agtrace_providers::normalize_codex_file_v2(path)
        .expect("Failed to parse Codex file with v2");

    assert!(!events.is_empty(), "Expected at least one event");

    // Snapshot only metadata field from all events in pretty JSON format with blank lines between
    let metadata_json_pretty = events
        .iter()
        .map(|e| serde_json::to_string_pretty(&e.metadata).unwrap())
        .collect::<Vec<_>>()
        .join("\n\n");
    insta::assert_snapshot!("codex_events_raw", metadata_json_pretty);
}

#[test]
fn test_claude_parse_raw_snapshot() {
    let path = Path::new("tests/samples/claude_session.jsonl");

    if !path.exists() {
        eprintln!("Warning: Test file not found, skipping: {}", path.display());
        return;
    }

    let events = agtrace_providers::normalize_claude_file_v2(path)
        .expect("Failed to parse Claude file with v2");

    assert!(!events.is_empty(), "Expected at least one event");

    // Snapshot only metadata field from all events in pretty JSON format with blank lines between
    let metadata_json_pretty = events
        .iter()
        .map(|e| serde_json::to_string_pretty(&e.metadata).unwrap())
        .collect::<Vec<_>>()
        .join("\n\n");
    insta::assert_snapshot!("claude_events_raw", metadata_json_pretty);
}
