mod common;
use common::TestFixture;

#[test]
fn test_session_list_filtering() {
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

    fixture.index_update().expect("Failed to index");

    // Test 1: List all sessions with JSON format
    let mut cmd = fixture.command();
    let output = cmd
        .arg("session")
        .arg("list")
        .arg("--format")
        .arg("json")
        .arg("--no-auto-refresh")
        .output()
        .expect("Failed to run session list");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).expect("Parse failed");
    let sessions = result["content"]["sessions"]
        .as_array()
        .expect("Expected sessions array in content");

    assert!(!sessions.is_empty(), "Expected at least 1 session");

    // Test 2: List with source filter
    let mut cmd = fixture.command();
    let output = cmd
        .arg("session")
        .arg("list")
        .arg("--format")
        .arg("json")
        .arg("--source")
        .arg("claude_code")
        .arg("--no-auto-refresh")
        .output()
        .expect("Failed to run session list with source filter");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).expect("Parse failed");
    let filtered = result["content"]["sessions"]
        .as_array()
        .expect("Expected sessions array in content");

    for session in filtered {
        let provider = session["provider"]
            .as_str()
            .expect("Session should have provider field");
        assert_eq!(provider, "claude_code", "Provider should be claude_code");
    }

    // Test 3: List with limit
    let mut cmd = fixture.command();
    let output = cmd
        .arg("session")
        .arg("list")
        .arg("--format")
        .arg("json")
        .arg("--limit")
        .arg("1")
        .arg("--no-auto-refresh")
        .output()
        .expect("Failed to run session list with limit");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).expect("Parse failed");
    let limited = result["content"]["sessions"]
        .as_array()
        .expect("Expected sessions array in content");

    assert!(limited.len() <= 1, "Limit should restrict results");

    // Test 4: Plain format output (should not be JSON)
    let mut cmd = fixture.command();
    let output = cmd
        .arg("session")
        .arg("list")
        .arg("--format")
        .arg("plain")
        .arg("--no-auto-refresh")
        .output()
        .expect("Failed to run session list with plain format");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.trim().starts_with('['),
        "Plain format should not be JSON"
    );
}
