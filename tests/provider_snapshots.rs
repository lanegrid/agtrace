use std::path::Path;

#[test]
fn test_gemini_parse_snapshot() {
    let path = Path::new("samples-tmp/.gemini/tmp/9126eddec7f67e038794657b4d517dd9cb5226468f30b5ee7296c27d65e84fde/chats/session-2025-12-09T19-50-f0a689a6.json");

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
    let path = Path::new("samples-tmp/.codex/sessions/2025/12/10/rollout-2025-12-10T04-55-16-019b04ae-b1c6-7c72-a134-a4c2de66058c.jsonl");

    if !path.exists() {
        eprintln!("Warning: Test file not found, skipping: {}", path.display());
        return;
    }

    let session_id = "test-session";
    let events = agtrace::providers::codex::io::normalize_codex_file(path, session_id, None)
        .expect("Failed to parse Codex file");

    assert!(!events.is_empty(), "Expected at least one event");

    let sample_events: Vec<_> = events.iter().take(3).collect();
    insta::assert_json_snapshot!("codex_events_sample", sample_events);

    assert_eq!(events[0].source, agtrace::model::Source::Codex);
}

#[test]
fn test_claude_parse_snapshot() {
    let path = Path::new("samples-tmp/.claude/projects/-Users-zawakin-agent-sample/7f2abd2d-7cfc-4447-9ddd-3ca8d14e02e9.jsonl");

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
