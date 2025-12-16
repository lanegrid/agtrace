mod fixtures;

use fixtures::TestFixture;

#[test]
fn test_index_scan_and_query() {
    let fixture = TestFixture::new();

    fixture
        .setup_provider("claude_code")
        .expect("Failed to setup provider");

    fixture
        .copy_sample_file("claude_session.jsonl", "session1.jsonl")
        .expect("Failed to copy sample 1");

    fixture
        .copy_sample_file("claude_agent.jsonl", "session2.jsonl")
        .expect("Failed to copy sample 2");

    fixture.index_update().expect("Failed to run index update");

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
    let sessions: serde_json::Value =
        serde_json::from_str(&stdout).expect("Failed to parse JSON output");

    assert!(
        sessions.is_array(),
        "Expected JSON array of sessions, got: {}",
        stdout
    );

    let sessions_array = sessions.as_array().unwrap();
    assert!(
        sessions_array.len() >= 1,
        "Expected at least 1 session, found {}",
        sessions_array.len()
    );

    for session in sessions_array {
        let session_id = session["id"]
            .as_str()
            .expect("Session should have string id");

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
            "session show {} failed: {}",
            session_id,
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
