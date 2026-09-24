use std::path::Path;

// Snapshot tests - test provider normalization
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

/// Diagnostics of the sample files: the samples are expected to decode cleanly.
#[test]
fn test_sample_diagnostics_snapshot() {
    use agtrace_providers::{ClaudeProvider, CodexProvider, DecodeOptions, decode_file};

    let (_, _, claude) = decode_file(
        &ClaudeProvider,
        Path::new("tests/samples/claude_session.jsonl"),
        DecodeOptions::default(),
    )
    .expect("claude sample decodes");
    let (_, _, codex) = decode_file(
        &CodexProvider,
        Path::new("tests/samples/codex_session.jsonl"),
        DecodeOptions::default(),
    )
    .expect("codex sample decodes");

    assert!(!claude.has_errors(), "{claude:?}");
    assert!(!codex.has_errors(), "{codex:?}");
    insta::assert_json_snapshot!("sample_diagnostics", (claude, codex));
}
