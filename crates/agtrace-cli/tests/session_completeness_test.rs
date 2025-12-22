mod common;
use common::TestFixture;

#[test]
fn test_session_includes_all_files_main_and_sidechain() {
    let fixture = TestFixture::new();

    fixture
        .setup_provider("claude_code")
        .expect("Failed to setup provider");

    // Copy both main session file and sidechain agent file to the project directory
    // Both files share the same sessionId: 7f2abd2d-7cfc-4447-9ddd-3ca8d14e02e9
    // and cwd: /Users/zawakin/agent-sample
    let project_dir = "/Users/zawakin/agent-sample";

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
        .arg("--json")
        .output()
        .expect("Failed to run session show");

    assert!(
        output.status.success(),
        "session show failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let events: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&output.stdout)).expect("Parse failed");
    let events_array = events.as_array().expect("Expected array");

    // Verify that we have events from both main and sidechain streams
    let mut has_main_events = false;
    let mut has_sidechain_events = false;

    for event in events_array {
        let stream_id = &event["stream_id"];

        // Check for main stream events
        if let Some(stream_type) = stream_id["stream_type"].as_str() {
            if stream_type == "main" {
                has_main_events = true;
            } else if stream_type == "sidechain" {
                // Verify it's the expected agent ID
                if let Some(agent_id) = stream_id["stream_data"]["agent_id"].as_str() {
                    assert_eq!(
                        agent_id, "0c4c3cf8",
                        "Expected sidechain agent_id to be 0c4c3cf8"
                    );
                    has_sidechain_events = true;
                }
            }
        }
    }

    // Ensure completeness: both main and sidechain events must be present
    assert!(
        has_main_events,
        "Completeness check failed: No main stream events found"
    );
    assert!(
        has_sidechain_events,
        "Completeness check failed: No sidechain events found (agent-*.jsonl not included)"
    );

    // Additional validation: count events from each stream
    let main_count = events_array
        .iter()
        .filter(|e| e["stream_id"]["stream_type"].as_str() == Some("main"))
        .count();

    let sidechain_count = events_array
        .iter()
        .filter(|e| e["stream_id"]["stream_type"].as_str() == Some("sidechain"))
        .count();

    assert!(
        main_count > 0,
        "Expected at least one main event, got {}",
        main_count
    );
    assert!(
        sidechain_count > 0,
        "Expected at least one sidechain event, got {}",
        sidechain_count
    );

    eprintln!(
        "✓ Completeness verified: {} main events + {} sidechain events = {} total events",
        main_count,
        sidechain_count,
        events_array.len()
    );
}
