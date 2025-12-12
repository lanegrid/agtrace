use std::path::Path;

#[test]
fn test_gemini_parse_snapshot() {
    let path = Path::new("tests/samples/gemini_session.json");

    if !path.exists() {
        eprintln!("Warning: Test file not found, skipping: {}", path.display());
        return;
    }

    let events =
        agtrace_providers::normalize_gemini_file(path).expect("Failed to parse Gemini file");

    assert!(!events.is_empty(), "Expected at least one event");

    // Snapshot all events in pretty JSON format with blank lines between events
    let json_pretty = events
        .iter()
        .map(|e| serde_json::to_string_pretty(e).unwrap())
        .collect::<Vec<_>>()
        .join("\n\n");
    insta::assert_snapshot!("gemini_events_sample", json_pretty);

    // Verify basic properties
    assert_eq!(events[0].source, agtrace_types::Source::Gemini);
    assert!(events[0].session_id.is_some());
}

#[test]
fn test_codex_parse_snapshot() {
    let path = Path::new("tests/samples/codex_session.jsonl");

    if !path.exists() {
        eprintln!("Warning: Test file not found, skipping: {}", path.display());
        return;
    }

    let events =
        agtrace_providers::normalize_codex_file(path, None).expect("Failed to parse Codex file");

    assert!(!events.is_empty(), "Expected at least one event");

    // Snapshot all events in pretty JSON format with blank lines between events
    let json_pretty = events
        .iter()
        .map(|e| serde_json::to_string_pretty(e).unwrap())
        .collect::<Vec<_>>()
        .join("\n\n");
    insta::assert_snapshot!("codex_events_sample", json_pretty);

    assert_eq!(events[0].source, agtrace_types::Source::Codex);
}

#[test]
fn test_claude_parse_snapshot() {
    let path = Path::new("tests/samples/claude_session.jsonl");

    if !path.exists() {
        eprintln!("Warning: Test file not found, skipping: {}", path.display());
        return;
    }

    let events =
        agtrace_providers::normalize_claude_file(path, None).expect("Failed to parse Claude file");

    assert!(!events.is_empty(), "Expected at least one event");

    // Snapshot all events in pretty JSON format with blank lines between events
    let json_pretty = events
        .iter()
        .map(|e| serde_json::to_string_pretty(e).unwrap())
        .collect::<Vec<_>>()
        .join("\n\n");
    insta::assert_snapshot!("claude_events_sample", json_pretty);

    assert_eq!(events[0].source, agtrace_types::Source::ClaudeCode);
}
