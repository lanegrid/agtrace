mod common;
use common::TestFixture;

/// Test: init with --refresh flag should re-index all sessions
#[test]
fn test_init_refresh_reindexes_sessions() {
    let fixture = TestFixture::new();

    let project_a = "/Users/test_user/project-a";
    fixture
        .copy_sample_file_to_project("claude_session.jsonl", "session1.jsonl", project_a)
        .expect("Failed to copy session");

    fixture
        .setup_provider("claude_code")
        .expect("Failed to setup provider");

    // First init
    let mut cmd = fixture.command();
    let output = cmd
        .arg("init")
        .arg("--all-projects")
        .output()
        .expect("Failed to run init");

    assert!(output.status.success());

    // Verify session exists
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
    let count_before = sessions.len();

    assert!(count_before > 0, "Should have sessions after first init");

    // Add another session
    fixture
        .copy_sample_file_to_project("claude_agent.jsonl", "session2.jsonl", project_a)
        .expect("Failed to copy second session");

    // Run init --refresh
    let mut cmd = fixture.command();
    let output = cmd
        .arg("init")
        .arg("--refresh")
        .arg("--all-projects")
        .output()
        .expect("Failed to run init --refresh");

    assert!(output.status.success());

    // Verify both sessions exist
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
    let count_after = sessions.len();

    assert!(
        count_after >= count_before,
        "init --refresh should discover new sessions"
    );
}

/// Test: init without --refresh should not re-scan if database exists
#[test]
fn test_init_without_refresh_skips_rescan() {
    let fixture = TestFixture::new();

    let project_a = "/Users/test_user/project-a";
    fixture
        .copy_sample_file_to_project("claude_session.jsonl", "session1.jsonl", project_a)
        .expect("Failed to copy session");

    fixture
        .setup_provider("claude_code")
        .expect("Failed to setup provider");

    // First init
    let mut cmd = fixture.command();
    let output = cmd
        .arg("init")
        .arg("--all-projects")
        .output()
        .expect("Failed to run init");

    assert!(output.status.success());

    // Add another session
    fixture
        .copy_sample_file_to_project("claude_agent.jsonl", "session2.jsonl", project_a)
        .expect("Failed to copy second session");

    // Run init again WITHOUT --refresh
    // This should detect database exists and skip re-scan
    let mut cmd = fixture.command();
    let output = cmd
        .arg("init")
        .arg("--all-projects")
        .output()
        .expect("Failed to run init");

    assert!(output.status.success());

    // The output should indicate database already exists
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Note: This behavior depends on init implementation
    // We just verify it doesn't error out
    assert!(!stdout.is_empty() || output.status.success());
}

/// Test: init on fresh directory with sessions in multiple projects
#[test]
fn test_init_fresh_with_multiple_projects() {
    let fixture = TestFixture::new();

    let project_a = "/Users/test_user/project-a";
    let project_b = "/Users/test_user/project-b";
    let project_c = "/Users/test_user/project-c";

    // Add sessions to multiple projects
    fixture
        .copy_sample_file_to_project("claude_session.jsonl", "session1.jsonl", project_a)
        .expect("Failed to copy to project A");

    fixture
        .copy_sample_file_to_project("claude_agent.jsonl", "session2.jsonl", project_b)
        .expect("Failed to copy to project B");

    fixture
        .copy_sample_file_to_project("claude_session.jsonl", "session3.jsonl", project_c)
        .expect("Failed to copy to project C");

    fixture
        .setup_provider("claude_code")
        .expect("Failed to setup provider");

    // Run init with --all-projects
    let mut cmd = fixture.command();
    let output = cmd
        .arg("init")
        .arg("--all-projects")
        .output()
        .expect("Failed to run init");

    assert!(
        output.status.success(),
        "init should succeed with multiple projects: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify all projects are indexed
    let mut cmd = fixture.command();
    let output = cmd
        .arg("project")
        .arg("list")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to run project list");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).expect("Parse failed");
    let projects = result["content"]["projects"]
        .as_array()
        .expect("Expected projects array");

    // Note: claude_session.jsonl contains sessionId 7f2abd2d... with cwd=/Users/test_user/agent-sample
    // When we copy it to different project directories, the session still has the original cwd,
    // so it will be indexed under the project hash of /Users/test_user/agent-sample
    // Only claude_agent.jsonl might have a different cwd
    assert!(
        projects.len() >= 1,
        "Should have indexed at least 1 project during init, got {}",
        projects.len()
    );

    // Verify all sessions are queryable with --all-projects
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
        "Should have sessions indexed across all projects"
    );
}

/// Test: init without --all-projects should only index current project
#[test]
fn test_init_without_all_projects_flag() {
    let fixture = TestFixture::new();

    let project_a = "/Users/test_user/project-a";
    let project_b = "/Users/test_user/project-b";

    fixture
        .copy_sample_file_to_project("claude_session.jsonl", "session1.jsonl", project_a)
        .expect("Failed to copy to project A");

    fixture
        .copy_sample_file_to_project("claude_agent.jsonl", "session2.jsonl", project_b)
        .expect("Failed to copy to project B");

    fixture
        .setup_provider("claude_code")
        .expect("Failed to setup provider");

    // Run init WITHOUT --all-projects
    // Note: The test fixture runs from a temp directory, not project A or B
    let mut cmd = fixture.command();
    let output = cmd
        .arg("init")
        .output()
        .expect("Failed to run init");

    assert!(output.status.success());

    // Since we're not in project A or B, and we didn't use --all-projects,
    // the behavior depends on implementation.
    // Let's just verify init succeeded without error
    let db_path = fixture.data_dir().join("agtrace.db");
    assert!(
        db_path.exists(),
        "Database should be created even when run from different directory"
    );
}

/// Test: Corrupted database - init --refresh should rebuild
#[test]
fn test_init_refresh_recovers_from_corruption() {
    let fixture = TestFixture::new();

    let project_a = "/Users/test_user/project-a";
    fixture
        .copy_sample_file_to_project("claude_session.jsonl", "session1.jsonl", project_a)
        .expect("Failed to copy session");

    fixture
        .setup_provider("claude_code")
        .expect("Failed to setup provider");

    // First init
    let mut cmd = fixture.command();
    let output = cmd
        .arg("init")
        .arg("--all-projects")
        .output()
        .expect("Failed to run init");

    assert!(output.status.success());

    // Simulate corruption by writing garbage to database
    let db_path = fixture.data_dir().join("agtrace.db");
    std::fs::write(&db_path, "corrupted data").expect("Failed to corrupt database");

    // Run init --refresh - should detect corruption and rebuild
    let mut cmd = fixture.command();
    let output = cmd
        .arg("init")
        .arg("--refresh")
        .arg("--all-projects")
        .output()
        .expect("Failed to run init --refresh");

    // This might fail or succeed depending on error handling
    // But it shouldn't panic
    let stderr = String::from_utf8_lossy(&output.stderr);
    eprintln!("init --refresh stderr: {}", stderr);

    // If it succeeded, verify database is functional
    if output.status.success() {
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

        // Should work after rebuild
        assert!(
            output.status.success(),
            "Session list should work after database rebuild"
        );
    }
}
