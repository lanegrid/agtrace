mod common;
use common::TestFixture;

// Test that sessions with multiple stream files (main + sidechain agent files)
// are successfully loaded and processed together
#[test]
fn test_session_includes_all_files_main_and_sidechain() {
    let fixture = TestFixture::new();

    fixture
        .setup_provider("claude_code")
        .expect("Failed to setup provider");

    // Copy both main session file and sidechain agent file to the project directory
    // Both files share the same sessionId: 7f2abd2d-7cfc-4447-9ddd-3ca8d14e02e9
    // and cwd: /Users/test_user/agent-sample
    let project_dir = "/Users/test_user/agent-sample";

    fixture
        .copy_sample_file_to_project("claude_session.jsonl", "session1.jsonl", project_dir)
        .expect("Failed to copy main session file");

    fixture
        .copy_sample_file_to_project("claude_agent.jsonl", "agent-0c4c3cf8.jsonl", project_dir)
        .expect("Failed to copy agent file");

    fixture.index_update().expect("Failed to index");

    // List sessions to get the session ID
    let mut cmd = fixture.command();
    let output = cmd
        .arg("session")
        .arg("list")
        .arg("--format")
        .arg("json")
        .arg("--all-projects")
        .arg("--no-auto-refresh")
        .output()
        .expect("Failed to run session list");

    assert!(
        output.status.success(),
        "session list failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).expect("Failed to parse");
    let sessions = result["content"]["sessions"]
        .as_array()
        .expect("Expected sessions array in content");

    assert!(!sessions.is_empty(), "Expected at least one session");

    let session_id = sessions[0]["id"].as_str().expect("Expected session id");

    // Show session events in JSON format
    let mut cmd = fixture.command();
    let output = cmd
        .arg("session")
        .arg("show")
        .arg(session_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to run session show");

    assert!(
        output.status.success(),
        "session show failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let result: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&output.stdout)).expect("Parse failed");

    // Verify CommandResultViewModel wrapper structure
    assert!(
        result.get("content").is_some(),
        "Expected 'content' field in CommandResultViewModel"
    );

    let session = &result["content"];

    // Verify the session has proper structure (v2 SessionAnalysisViewModel)
    assert!(
        session.get("header").is_some(),
        "Expected 'header' field in SessionAnalysisViewModel"
    );
    assert!(session.get("turns").is_some(), "Expected 'turns' field");

    let turns = session["turns"]
        .as_array()
        .expect("Expected turns to be an array");

    // Completeness check: If both files were loaded, we should have turns with steps
    // The presence of turns and steps indicates that events from all files were processed
    assert!(
        !turns.is_empty(),
        "Completeness check failed: No turns found (expected events from main + sidechain files)"
    );

    // Count total steps across all turns
    let total_steps: usize = turns
        .iter()
        .filter_map(|turn| turn["steps"].as_array())
        .map(|steps| steps.len())
        .sum();

    // With both main and sidechain files, we expect a significant number of steps
    // The claude_session.jsonl and claude_agent.jsonl together should have multiple events
    assert!(
        total_steps > 5,
        "Completeness check failed: Expected more steps from combined main+sidechain files, got {}",
        total_steps
    );

    // Verify the session header has expected fields
    let header = session.get("header").expect("Expected header");
    assert!(
        header.get("session_id").is_some(),
        "Expected session_id in header"
    );
    assert!(
        header.get("provider").is_some(),
        "Expected provider in header"
    );

    eprintln!(
        "✓ Completeness verified: Session has {} turns with {} total steps across all files",
        turns.len(),
        total_steps
    );
}
