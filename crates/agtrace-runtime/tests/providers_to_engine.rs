// Integration tests for the complete flow: raw provider data → normalized events → engine processing
use agtrace_engine::assemble_session;
use std::path::Path;

#[test]
fn test_claude_end_to_end() {
    let path = Path::new("../../agtrace-providers/tests/samples/claude_session.jsonl");

    if !path.exists() {
        eprintln!("Warning: Test file not found, skipping: {}", path.display());
        return;
    }

    // providers: normalize raw data → AgentEvent[]
    let events =
        agtrace_providers::normalize_claude_file(path).expect("Failed to normalize Claude file");

    assert!(!events.is_empty(), "Expected at least one event");

    // engine: assemble session
    let session = assemble_session(&events).expect("Failed to assemble session");
    assert!(!session.turns.is_empty(), "Expected at least one turn");
}

#[test]
fn test_codex_end_to_end() {
    let path = Path::new("../../agtrace-providers/tests/samples/codex_session.jsonl");

    if !path.exists() {
        eprintln!("Warning: Test file not found, skipping: {}", path.display());
        return;
    }

    // providers: normalize raw data → AgentEvent[]
    let events =
        agtrace_providers::normalize_codex_file(path).expect("Failed to normalize Codex file");

    assert!(!events.is_empty(), "Expected at least one event");

    // engine: assemble session
    let session = assemble_session(&events).expect("Failed to assemble session");
    assert!(!session.turns.is_empty(), "Expected at least one turn");
}

#[test]
fn test_gemini_end_to_end() {
    let path = Path::new("../../agtrace-providers/tests/samples/gemini_session.json");

    if !path.exists() {
        eprintln!("Warning: Test file not found, skipping: {}", path.display());
        return;
    }

    // providers: normalize raw data → AgentEvent[]
    let events =
        agtrace_providers::normalize_gemini_file(path).expect("Failed to normalize Gemini file");

    assert!(!events.is_empty(), "Expected at least one event");

    // engine: assemble session
    let session = assemble_session(&events).expect("Failed to assemble session");
    assert!(!session.turns.is_empty(), "Expected at least one turn");
}
