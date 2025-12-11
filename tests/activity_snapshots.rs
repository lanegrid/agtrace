use agtrace::core::aggregate_activities;
use std::path::Path;

#[test]
fn test_claude_activity_aggregation() {
    let path = Path::new("tests/samples/claude_session.jsonl");

    if !path.exists() {
        eprintln!("Warning: Test file not found, skipping: {}", path.display());
        return;
    }

    let events = agtrace::providers::claude::io::normalize_claude_file(path, None)
        .expect("Failed to parse Claude file");

    let activities = aggregate_activities(&events);

    assert!(!activities.is_empty(), "Expected at least one activity");

    let json = serde_json::to_string_pretty(&activities).unwrap();
    insta::assert_snapshot!("claude_activities", json);
}

#[test]
fn test_codex_activity_aggregation() {
    let path = Path::new("tests/samples/codex_session.jsonl");

    if !path.exists() {
        eprintln!("Warning: Test file not found, skipping: {}", path.display());
        return;
    }

    let events = agtrace::providers::codex::io::normalize_codex_file(path, None)
        .expect("Failed to parse Codex file");

    let activities = aggregate_activities(&events);

    assert!(!activities.is_empty(), "Expected at least one activity");

    let json = serde_json::to_string_pretty(&activities).unwrap();
    insta::assert_snapshot!("codex_activities", json);
}

#[test]
fn test_gemini_activity_aggregation() {
    let path = Path::new("tests/samples/gemini_session.json");

    if !path.exists() {
        eprintln!("Warning: Test file not found, skipping: {}", path.display());
        return;
    }

    let events = agtrace::providers::gemini::normalize_gemini_file(path)
        .expect("Failed to parse Gemini file");

    let activities = aggregate_activities(&events);

    assert!(!activities.is_empty(), "Expected at least one activity");

    let json = serde_json::to_string_pretty(&activities).unwrap();
    insta::assert_snapshot!("gemini_activities", json);
}
