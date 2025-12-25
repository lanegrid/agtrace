mod common;
use common::TestFixture;

/// Test: Empty database with no sessions - session list should return empty
#[test]
fn test_empty_database_session_list() {
    let fixture = TestFixture::new();

    fixture
        .setup_provider("claude_code")
        .expect("Failed to setup provider");

    // Don't add any sessions - just index
    fixture.index_update().expect("Failed to index empty log directory");

    // List sessions - should return empty array
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
        "session list should succeed on empty database: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).expect("Parse failed");
    let sessions = result["content"]["sessions"]
        .as_array()
        .expect("Expected sessions array");

    assert_eq!(sessions.len(), 0, "Empty database should have 0 sessions");

    // Verify badge/message indicates no sessions
    let badge = &result["badge"];
    assert!(badge.is_object(), "Should have badge field");
}

/// Test: Empty database - project list should return empty
#[test]
fn test_empty_database_project_list() {
    let fixture = TestFixture::new();

    fixture
        .setup_provider("claude_code")
        .expect("Failed to setup provider");

    fixture.index_update().expect("Failed to index");

    let mut cmd = fixture.command();
    let output = cmd
        .arg("project")
        .arg("list")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to run project list");

    assert!(
        output.status.success(),
        "project list should succeed on empty database: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).expect("Parse failed");
    let projects = result["content"]["projects"]
        .as_array()
        .expect("Expected projects array");

    assert_eq!(projects.len(), 0, "Empty database should have 0 projects");
}

/// Test: Init with no sessions should work without error
#[test]
fn test_init_with_no_sessions() {
    let fixture = TestFixture::new();

    fixture
        .setup_provider("claude_code")
        .expect("Failed to setup provider");

    // Run init without any session files
    let mut cmd = fixture.command();
    let output = cmd
        .arg("init")
        .arg("--all-projects")
        .output()
        .expect("Failed to run init");

    assert!(
        output.status.success(),
        "init should succeed with no sessions: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify database was created
    let db_path = fixture.data_dir().join("agtrace.db");
    assert!(
        db_path.exists(),
        "Database should be created even with no sessions"
    );

    // Verify we can list sessions (should be empty)
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

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).expect("Parse failed");
    let sessions = result["content"]["sessions"]
        .as_array()
        .expect("Expected sessions array");

    assert_eq!(sessions.len(), 0);
}

/// Test: Init with sessions should index them correctly
#[test]
fn test_init_with_existing_sessions() {
    let fixture = TestFixture::new();

    // Add sessions BEFORE setup_provider (before init)
    let project_a = "/Users/test_user/project-a";
    fixture
        .copy_sample_file_to_project("claude_session.jsonl", "session1.jsonl", project_a)
        .expect("Failed to copy session");

    fixture
        .setup_provider("claude_code")
        .expect("Failed to setup provider");

    // Run init - should index the existing sessions
    let mut cmd = fixture.command();
    let output = cmd
        .arg("init")
        .arg("--all-projects")
        .output()
        .expect("Failed to run init");

    assert!(
        output.status.success(),
        "init should succeed and index sessions: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify sessions were indexed
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

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).expect("Parse failed");
    let sessions = result["content"]["sessions"]
        .as_array()
        .expect("Expected sessions array");

    assert!(
        !sessions.is_empty(),
        "Init should have indexed the existing session"
    );
}

/// Test: Index update is idempotent - running twice doesn't duplicate
#[test]
fn test_index_update_idempotent() {
    let fixture = TestFixture::new();

    fixture
        .setup_provider("claude_code")
        .expect("Failed to setup provider");

    let project_a = "/Users/test_user/project-a";
    fixture
        .copy_sample_file_to_project("claude_session.jsonl", "session1.jsonl", project_a)
        .expect("Failed to copy session");

    // Index once
    fixture.index_update().expect("Failed first index");

    // Count sessions
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

    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).expect("Parse failed");
    let sessions_first = result["content"]["sessions"]
        .as_array()
        .expect("Expected sessions array");
    let count_first = sessions_first.len();

    // Index again
    fixture.index_update().expect("Failed second index");

    // Count sessions again - should be same
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

    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).expect("Parse failed");
    let sessions_second = result["content"]["sessions"]
        .as_array()
        .expect("Expected sessions array");
    let count_second = sessions_second.len();

    assert_eq!(
        count_first, count_second,
        "Index update should be idempotent - running twice should not duplicate sessions"
    );
}

/// Test: Index rebuild vs update - rebuild scans all files
#[test]
fn test_index_rebuild_vs_update() {
    let fixture = TestFixture::new();

    fixture
        .setup_provider("claude_code")
        .expect("Failed to setup provider");

    let project_a = "/Users/test_user/project-a";
    fixture
        .copy_sample_file_to_project("claude_session.jsonl", "session1.jsonl", project_a)
        .expect("Failed to copy session");

    // First index
    fixture.index_update().expect("Failed first index");

    // Add another session
    fixture
        .copy_sample_file_to_project("claude_agent.jsonl", "session2.jsonl", project_a)
        .expect("Failed to copy second session");

    // Run update (incremental) - should find new session
    let mut cmd = fixture.command();
    let output = cmd
        .arg("index")
        .arg("update")
        .arg("--all-projects")
        .output()
        .expect("Failed to run index update");

    assert!(output.status.success());

    // Count sessions
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

    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).expect("Parse failed");
    let sessions = result["content"]["sessions"]
        .as_array()
        .expect("Expected sessions array");

    assert!(
        sessions.len() >= 1,
        "Should have at least 1 session after incremental update"
    );

    // Run rebuild - should scan all files again
    let mut cmd = fixture.command();
    let output = cmd
        .arg("index")
        .arg("rebuild")
        .arg("--all-projects")
        .output()
        .expect("Failed to run index rebuild");

    assert!(output.status.success());

    // Count should remain the same
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

    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).expect("Parse failed");
    let sessions_after_rebuild = result["content"]["sessions"]
        .as_array()
        .expect("Expected sessions array");

    assert_eq!(
        sessions.len(),
        sessions_after_rebuild.len(),
        "Rebuild should maintain same session count as update"
    );
}

/// Test: Auto-refresh works correctly when listing sessions
#[test]
fn test_session_list_auto_refresh() {
    let fixture = TestFixture::new();

    fixture
        .setup_provider("claude_code")
        .expect("Failed to setup provider");

    let project_a = "/Users/test_user/project-a";

    // Don't run index_update explicitly
    // Just add a session file
    fixture
        .copy_sample_file_to_project("claude_session.jsonl", "session1.jsonl", project_a)
        .expect("Failed to copy session");

    // List sessions WITHOUT --no-auto-refresh
    // This should trigger auto-refresh and find the session
    let mut cmd = fixture.command();
    let output = cmd
        .arg("session")
        .arg("list")
        .arg("--format")
        .arg("json")
        .arg("--all-projects")
        .output()
        .expect("Failed to run session list with auto-refresh");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).expect("Parse failed");
    let sessions = result["content"]["sessions"]
        .as_array()
        .expect("Expected sessions array");

    assert!(
        !sessions.is_empty(),
        "Auto-refresh should have discovered the session without explicit index update"
    );
}
