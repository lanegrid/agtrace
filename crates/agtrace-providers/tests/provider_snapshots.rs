use std::path::Path;

// Snapshot tests - test provider normalization
#[test]
fn test_gemini_parse_snapshot() {
    let path = Path::new("tests/samples/gemini_session.json");

    if !path.exists() {
        eprintln!("Warning: Test file not found, skipping: {}", path.display());
        return;
    }

    let events = agtrace_providers::normalize_gemini_file(path)
        .expect("Failed to parse Gemini file successfully");

    assert!(!events.is_empty(), "Expected at least one event");

    // Snapshot all events in pretty JSON format with deterministic UUIDs
    let json_pretty = events
        .iter()
        .map(|e| serde_json::to_string_pretty(e).unwrap())
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

    let events = agtrace_providers::normalize_codex_file(path)
        .expect("Failed to parse Codex file successfully");

    assert!(!events.is_empty(), "Expected at least one event");

    // Snapshot all events in pretty JSON format with deterministic UUIDs
    let json_pretty = events
        .iter()
        .map(|e| serde_json::to_string_pretty(e).unwrap())
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

    let events = agtrace_providers::normalize_claude_file(path)
        .expect("Failed to parse Claude file successfully");

    assert!(!events.is_empty(), "Expected at least one event");

    // Snapshot all events in pretty JSON format with deterministic UUIDs
    let json_pretty = events
        .iter()
        .map(|e| serde_json::to_string_pretty(e).unwrap())
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

    let events = agtrace_providers::normalize_gemini_file(path)
        .expect("Failed to parse Gemini file successfully");

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

    let events = agtrace_providers::normalize_codex_file(path)
        .expect("Failed to parse Codex file successfully");

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

    let events = agtrace_providers::normalize_claude_file(path)
        .expect("Failed to parse Claude file successfully");

    assert!(!events.is_empty(), "Expected at least one event");

    // Snapshot only metadata field from all events in pretty JSON format with blank lines between
    let metadata_json_pretty = events
        .iter()
        .map(|e| serde_json::to_string_pretty(&e.metadata).unwrap())
        .collect::<Vec<_>>()
        .join("\n\n");
    insta::assert_snapshot!("claude_events_raw", metadata_json_pretty);
}
