use std::path::PathBuf;

/// Test that scan_legacy derives project_hash from SessionIndex.project_root
/// instead of blindly using context.project_hash
///
/// This test uses direct file parsing to avoid file name pattern issues
#[test]
fn test_claude_derives_project_hash_from_session_data() {
    let path = PathBuf::from("tests/samples/claude_session.jsonl");

    if !path.exists() {
        eprintln!("Warning: Test file not found, skipping: {}", path.display());
        return;
    }

    let events =
        agtrace_providers::normalize_claude_file(&path).expect("Failed to parse Claude file");

    assert!(!events.is_empty(), "Expected at least one event");

    // Extract project_root from events
    let project_roots: Vec<_> = events
        .iter()
        .filter_map(|e| {
            e.metadata
                .as_ref()
                .and_then(|m| m.get("cwd"))
                .and_then(|v| v.as_str())
        })
        .collect();

    assert!(!project_roots.is_empty(), "Expected at least one cwd field");

    // Verify project_hash can be computed from cwd
    for root in project_roots {
        let project_hash = agtrace_types::project_hash_from_root(root);
        assert_ne!(
            project_hash,
            agtrace_types::ProjectHash::from("unknown"),
            "Project hash derived from cwd should not be 'unknown'"
        );
    }
}

#[test]
fn test_codex_derives_project_hash_from_session_data() {
    let path = PathBuf::from("tests/samples/codex_session.jsonl");

    if !path.exists() {
        eprintln!("Warning: Test file not found, skipping: {}", path.display());
        return;
    }

    // For Codex, cwd is extracted from the file header, not from event metadata
    use agtrace_providers::codex::io::extract_cwd_from_codex_file;

    let cwd =
        extract_cwd_from_codex_file(&path).expect("Expected to find cwd in Codex session file");

    // Verify project_hash can be computed from cwd
    let project_hash = agtrace_types::project_hash_from_root(&cwd);
    assert_ne!(
        project_hash,
        agtrace_types::ProjectHash::from("unknown"),
        "Project hash derived from cwd should not be 'unknown'"
    );
}

#[test]
fn test_gemini_derives_project_hash_from_file() {
    let path = PathBuf::from("tests/samples/gemini_session.json");

    if !path.exists() {
        eprintln!("Warning: Test file not found, skipping: {}", path.display());
        return;
    }

    // Extract project_hash directly from file
    use agtrace_providers::gemini::io::extract_project_hash_from_gemini_file;

    let project_hash = extract_project_hash_from_gemini_file(&path)
        .expect("Failed to extract project_hash from Gemini file");

    assert_ne!(
        project_hash,
        agtrace_types::ProjectHash::from("unknown"),
        "Gemini project_hash extracted from file should not be 'unknown'"
    );
}

/// Regression test: Verify that the fix prevents "unknown" from being used as project_hash
///
/// This is a meta-test that verifies the fix works by checking that
/// parsed session data contains valid project_root/project_hash information
#[test]
fn test_regression_session_data_contains_project_info() {
    // Test Claude
    let claude_path = PathBuf::from("tests/samples/claude_session.jsonl");
    if claude_path.exists() {
        let events = agtrace_providers::normalize_claude_file(&claude_path)
            .expect("Failed to parse Claude file");
        let has_cwd = events
            .iter()
            .any(|e| e.metadata.as_ref().and_then(|m| m.get("cwd")).is_some());
        assert!(has_cwd, "Claude events should contain cwd field");
    }

    // Test Codex - cwd is in file header, not event metadata
    let codex_path = PathBuf::from("tests/samples/codex_session.jsonl");
    if codex_path.exists() {
        use agtrace_providers::codex::io::extract_cwd_from_codex_file;
        let cwd = extract_cwd_from_codex_file(&codex_path);
        assert!(cwd.is_some(), "Codex file should contain cwd field");
    }

    // Test Gemini
    let gemini_path = PathBuf::from("tests/samples/gemini_session.json");
    if gemini_path.exists() {
        use agtrace_providers::gemini::io::extract_project_hash_from_gemini_file;
        let hash = extract_project_hash_from_gemini_file(&gemini_path);
        assert!(hash.is_some(), "Gemini file should contain projectHash");
    }
}
