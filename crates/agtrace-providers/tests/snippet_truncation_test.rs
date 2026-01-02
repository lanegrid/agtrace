use std::path::PathBuf;

/// Test that Claude snippet extraction truncates long messages to 200 chars
#[test]
fn test_claude_snippet_truncation() {
    use agtrace_providers::claude::io::extract_claude_header;

    let path = PathBuf::from("tests/samples/claude_session.jsonl");

    if !path.exists() {
        eprintln!("Warning: Test file not found, skipping: {}", path.display());
        return;
    }

    let header = extract_claude_header(&path).expect("Failed to extract Claude header");

    if let Some(snippet) = header.snippet {
        // Verify snippet is truncated (200 chars + "...(truncated)" = max 214 chars)
        assert!(
            snippet.chars().count() <= 214,
            "Snippet should be truncated to max 214 chars, got {} chars",
            snippet.chars().count()
        );

        // If original message was longer than 200 chars, it should be truncated
        if snippet.contains("...(truncated)") {
            assert!(
                snippet.chars().count() > 200,
                "Truncated snippets should have more than 200 chars (including suffix)"
            );
            assert!(
                snippet.ends_with("...(truncated)"),
                "Truncated snippets should end with '...(truncated)'"
            );
        }
    }
}

/// Test that Codex snippet extraction truncates long messages to 200 chars
#[test]
fn test_codex_snippet_truncation() {
    use agtrace_providers::codex::io::extract_codex_header;

    let path = PathBuf::from("tests/samples/codex_session.jsonl");

    if !path.exists() {
        eprintln!("Warning: Test file not found, skipping: {}", path.display());
        return;
    }

    let header = extract_codex_header(&path).expect("Failed to extract Codex header");

    if let Some(snippet) = header.snippet {
        // Verify snippet is truncated (200 chars + "...(truncated)" = max 214 chars)
        assert!(
            snippet.chars().count() <= 214,
            "Snippet should be truncated to max 214 chars, got {} chars",
            snippet.chars().count()
        );

        // If original message was longer than 200 chars, it should be truncated
        if snippet.contains("...(truncated)") {
            assert!(
                snippet.chars().count() > 200,
                "Truncated snippets should have more than 200 chars (including suffix)"
            );
            assert!(
                snippet.ends_with("...(truncated)"),
                "Truncated snippets should end with '...(truncated)'"
            );
        }
    }
}

/// Test that Gemini snippet extraction truncates long messages to 200 chars
#[test]
fn test_gemini_snippet_truncation() {
    use agtrace_providers::gemini::io::extract_gemini_header;

    let path = PathBuf::from("tests/samples/gemini_session.json");

    if !path.exists() {
        eprintln!("Warning: Test file not found, skipping: {}", path.display());
        return;
    }

    let header = extract_gemini_header(&path).expect("Failed to extract Gemini header");

    if let Some(snippet) = header.snippet {
        // Verify snippet is truncated (200 chars + "...(truncated)" = max 214 chars)
        assert!(
            snippet.chars().count() <= 214,
            "Snippet should be truncated to max 214 chars, got {} chars",
            snippet.chars().count()
        );

        // If original message was longer than 200 chars, it should be truncated
        if snippet.contains("...(truncated)") {
            assert!(
                snippet.chars().count() > 200,
                "Truncated snippets should have more than 200 chars (including suffix)"
            );
            assert!(
                snippet.ends_with("...(truncated)"),
                "Truncated snippets should end with '...(truncated)'"
            );
        }
    }
}

/// Test UTF-8 safety: truncation should not break multi-byte characters
#[test]
fn test_utf8_safety_in_truncation() {
    use agtrace_types::truncate;

    // Japanese text with multi-byte chars
    let japanese = "これは日本語のテストです。".repeat(20); // ~500 chars
    let truncated = truncate(&japanese, 200);

    // Should be valid UTF-8
    assert!(truncated.is_ascii() || std::str::from_utf8(truncated.as_bytes()).is_ok());

    // Should be truncated
    assert!(truncated.chars().count() <= 214);
    assert!(truncated.ends_with("...(truncated)"));

    // Emoji test
    let emoji = "🎉🎊🎈".repeat(100); // ~300 chars
    let truncated = truncate(&emoji, 200);

    // Should be valid UTF-8
    assert!(std::str::from_utf8(truncated.as_bytes()).is_ok());
    assert!(truncated.chars().count() <= 214);
}
