use agtrace_engine::interpret_turns;
use std::path::Path;

#[test]
fn test_claude_turn_interpretation() {
    let path = Path::new("tests/samples/claude_session.jsonl");

    if !path.exists() {
        eprintln!("Warning: Test file not found, skipping: {}", path.display());
        return;
    }

    let events =
        agtrace_providers::normalize_claude_file(path, None).expect("Failed to parse Claude file");

    let turns = interpret_turns(&events);

    assert!(!turns.is_empty(), "Expected at least one turn");

    let json = serde_json::to_string_pretty(&turns).unwrap();
    insta::assert_snapshot!("claude_turns", json);
}

#[test]
fn test_codex_turn_interpretation() {
    let path = Path::new("tests/samples/codex_session.jsonl");

    if !path.exists() {
        eprintln!("Warning: Test file not found, skipping: {}", path.display());
        return;
    }

    let events =
        agtrace_providers::normalize_codex_file(path, None).expect("Failed to parse Codex file");

    let turns = interpret_turns(&events);

    assert!(!turns.is_empty(), "Expected at least one turn");

    let json = serde_json::to_string_pretty(&turns).unwrap();
    insta::assert_snapshot!("codex_turns", json);
}

#[test]
fn test_gemini_turn_interpretation() {
    let path = Path::new("tests/samples/gemini_session.json");

    if !path.exists() {
        eprintln!("Warning: Test file not found, skipping: {}", path.display());
        return;
    }

    let events =
        agtrace_providers::normalize_gemini_file(path).expect("Failed to parse Gemini file");

    let turns = interpret_turns(&events);

    assert!(!turns.is_empty(), "Expected at least one turn");

    let json = serde_json::to_string_pretty(&turns).unwrap();
    insta::assert_snapshot!("gemini_turns", json);
}
