mod fixtures;

use fixtures::TestFixture;

#[test]
fn test_init_full_workflow() {
    let fixture = TestFixture::new();

    fixture
        .copy_sample_file("claude_session.jsonl", "session1.jsonl")
        .expect("Failed to copy sample file");

    fixture
        .setup_provider("claude_code")
        .expect("Failed to setup provider");

    let config_path = fixture.data_dir().join("config.toml");
    assert!(
        config_path.exists(),
        "Config file should be created at {}",
        config_path.display()
    );

    let mut cmd = fixture.command();
    let output = cmd
        .arg("init")
        .arg("--all-projects")
        .output()
        .expect("Failed to run init");

    assert!(
        output.status.success(),
        "init command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let db_path = fixture.data_dir().join("agtrace.db");
    assert!(
        db_path.exists(),
        "Database should be created at {}",
        db_path.display()
    );

    let mut cmd = fixture.command();
    let output = cmd
        .arg("session")
        .arg("list")
        .arg("--format")
        .arg("json")
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

    assert!(sessions.is_array(), "Expected JSON array, got: {}", stdout);

    let sessions_array = sessions.as_array().unwrap();
    assert!(
        !sessions_array.is_empty(),
        "Expected at least one session to be indexed"
    );
}
