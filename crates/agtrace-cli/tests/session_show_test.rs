mod fixtures;

use fixtures::TestFixture;

#[test]
fn test_session_show_filtering() {
    let fixture = TestFixture::new();

    fixture
        .setup_provider("claude_code")
        .expect("Failed to setup provider");

    fixture
        .copy_sample_file("claude_session.jsonl", "session1.jsonl")
        .expect("Failed to copy sample file");

    fixture.index_update().expect("Failed to index");

    let mut cmd = fixture.command();
    let output = cmd
        .arg("session")
        .arg("list")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to run session list");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let sessions: serde_json::Value = serde_json::from_str(&stdout).expect("Failed to parse");
    let sessions_array = sessions.as_array().expect("Expected array");

    assert!(!sessions_array.is_empty(), "Expected at least one session");

    let session_id = sessions_array[0]["id"]
        .as_str()
        .expect("Expected session id");

    let mut cmd = fixture.command();
    let output = cmd
        .arg("session")
        .arg("show")
        .arg(session_id)
        .arg("--json")
        .output()
        .expect("Failed to run session show");

    assert!(output.status.success());
    let all_events: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&output.stdout)).expect("Parse failed");
    let all_events_array = all_events.as_array().expect("Expected array");
    let total_events = all_events_array.len();

    assert!(total_events > 0, "Expected at least one event");

    let mut cmd = fixture.command();
    let output = cmd
        .arg("session")
        .arg("show")
        .arg(session_id)
        .arg("--json")
        .arg("--hide")
        .arg("text")
        .output()
        .expect("Failed to run session show with --hide");

    assert!(output.status.success());
    let filtered: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&output.stdout)).expect("Parse failed");
    let filtered_array = filtered.as_array().expect("Expected array");

    for event in filtered_array {
        let payload = &event["payload"];
        if let Some(payload_obj) = payload.as_object() {
            assert!(
                !payload_obj.contains_key("Text"),
                "Text events should be filtered out"
            );
        }
    }

    assert!(
        filtered_array.len() <= total_events,
        "Filtered count should not exceed total"
    );

    let mut cmd = fixture.command();
    let output = cmd
        .arg("session")
        .arg("show")
        .arg(session_id)
        .arg("--json")
        .arg("--only")
        .arg("tool_use")
        .output()
        .expect("Failed to run session show with --only");

    assert!(output.status.success());
    let only: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&output.stdout)).expect("Parse failed");
    let only_array = only.as_array().expect("Expected array");

    for event in only_array {
        let payload = &event["payload"];
        if let Some(payload_obj) = payload.as_object() {
            assert!(
                payload_obj.contains_key("ToolUse"),
                "Only ToolUse events should remain"
            );
        }
    }
}
