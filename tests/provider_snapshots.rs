use std::path::Path;

#[test]
fn test_gemini_parse_snapshot() {
    let path = Path::new("tests/samples/gemini_session.json");

    if !path.exists() {
        eprintln!("Warning: Test file not found, skipping: {}", path.display());
        return;
    }

    let events = agtrace::providers::gemini::normalize_gemini_file(path)
        .expect("Failed to parse Gemini file");

    assert!(!events.is_empty(), "Expected at least one event");

    // Snapshot the first few events for verification
    let sample_events: Vec<_> = events.iter().take(3).collect();
    insta::assert_json_snapshot!("gemini_events_sample", sample_events);

    // Verify basic properties
    assert_eq!(events[0].source, agtrace::model::Source::Gemini);
    assert!(events[0].session_id.is_some());
}

#[test]
fn test_codex_parse_snapshot() {
    let path = Path::new("tests/samples/codex_session.jsonl");

    if !path.exists() {
        eprintln!("Warning: Test file not found, skipping: {}", path.display());
        return;
    }

    let events = agtrace::providers::codex::io::normalize_codex_file(path, None)
        .expect("Failed to parse Codex file");

    assert!(!events.is_empty(), "Expected at least one event");

    let sample_events: Vec<_> = events.iter().take(3).collect();
    insta::assert_json_snapshot!("codex_events_sample", sample_events);

    assert_eq!(events[0].source, agtrace::model::Source::Codex);
}

#[test]
fn test_claude_parse_snapshot() {
    let path = Path::new("tests/samples/claude_session.jsonl");

    if !path.exists() {
        eprintln!("Warning: Test file not found, skipping: {}", path.display());
        return;
    }

    let events = agtrace::providers::claude::io::normalize_claude_file(path, None)
        .expect("Failed to parse Claude file");

    assert!(!events.is_empty(), "Expected at least one event");

    let sample_events: Vec<_> = events.iter().take(3).collect();
    insta::assert_json_snapshot!("claude_events_sample", sample_events);

    assert_eq!(events[0].source, agtrace::model::Source::ClaudeCode);
}
