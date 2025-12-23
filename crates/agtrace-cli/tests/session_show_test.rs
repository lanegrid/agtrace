mod common;
use common::TestFixture;

#[test]
fn test_session_show_basic_structure() {
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
        .arg("--no-auto-refresh")
        .output()
        .expect("Failed to run session list");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).expect("Failed to parse");
    let sessions = result["content"]["sessions"]
        .as_array()
        .expect("Expected sessions array in content");

    assert!(!sessions.is_empty(), "Expected at least one session");

    let session_id = sessions[0]["id"].as_str().expect("Expected session id");

    // Test JSON output format - should be SessionAnalysisViewModel structure
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

    // Verify v2 SessionAnalysisViewModel structure
    assert!(
        session.get("header").is_some(),
        "Expected 'header' field in SessionAnalysisViewModel"
    );
    assert!(
        session.get("context_summary").is_some(),
        "Expected 'context_summary' field"
    );
    assert!(session.get("turns").is_some(), "Expected 'turns' field");

    let turns = session["turns"]
        .as_array()
        .expect("Expected turns to be an array");
    assert!(!turns.is_empty(), "Expected at least one turn");

    // Verify turn structure
    let first_turn = &turns[0];
    assert!(
        first_turn.get("turn_number").is_some(),
        "Expected turn_number"
    );
    assert!(first_turn.get("steps").is_some(), "Expected steps array");

    let steps = first_turn["steps"]
        .as_array()
        .expect("Expected steps to be an array");
    assert!(!steps.is_empty(), "Expected at least one step in the turn");

    // Test plain text output works
    let mut cmd = fixture.command();
    let output = cmd
        .arg("session")
        .arg("show")
        .arg(session_id)
        .arg("--format")
        .arg("plain")
        .output()
        .expect("Failed to run session show with plain format");

    assert!(
        output.status.success(),
        "plain format output failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let plain_output = String::from_utf8_lossy(&output.stdout);
    // Verify we got meaningful output (contains turn or session info)
    assert!(
        !plain_output.is_empty() && plain_output.len() > 50,
        "Expected substantial plain output, got {} bytes",
        plain_output.len()
    );
}
