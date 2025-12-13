use agtrace_engine::build_spans;
use std::path::Path;

#[test]
fn test_claude_span_building() {
    let path = Path::new("tests/samples/claude_session.jsonl");

    if !path.exists() {
        eprintln!("Warning: Test file not found, skipping: {}", path.display());
        return;
    }

    let events =
        agtrace_providers::normalize_claude_file(path, None).expect("Failed to parse Claude file");

    let spans = build_spans(&events);

    assert!(!spans.is_empty(), "Expected at least one span");

    let json = serde_json::to_string_pretty(&spans).unwrap();
    insta::assert_snapshot!("claude_spans", json);
}

#[test]
fn test_codex_span_building() {
    let path = Path::new("tests/samples/codex_session.jsonl");

    if !path.exists() {
        eprintln!("Warning: Test file not found, skipping: {}", path.display());
        return;
    }

    let events =
        agtrace_providers::normalize_codex_file(path, None).expect("Failed to parse Codex file");

    let spans = build_spans(&events);

    assert!(!spans.is_empty(), "Expected at least one span");

    let json = serde_json::to_string_pretty(&spans).unwrap();
    insta::assert_snapshot!("codex_spans", json);
}

#[test]
fn test_gemini_span_building() {
    let path = Path::new("tests/samples/gemini_session.json");

    if !path.exists() {
        eprintln!("Warning: Test file not found, skipping: {}", path.display());
        return;
    }

    let events =
        agtrace_providers::normalize_gemini_file(path).expect("Failed to parse Gemini file");

    let spans = build_spans(&events);

    assert!(!spans.is_empty(), "Expected at least one span");

    let json = serde_json::to_string_pretty(&spans).unwrap();
    insta::assert_snapshot!("gemini_spans", json);
}
