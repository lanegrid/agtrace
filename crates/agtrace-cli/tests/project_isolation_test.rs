mod common;
use common::TestFixture;

/// Test: Current project has sessions, list without --all-projects shows only current project
#[test]
fn test_project_isolation_current_project_only() {
    let fixture = TestFixture::new();

    fixture
        .setup_provider("claude_code")
        .expect("Failed to setup provider");

    // Note: Sample files have embedded cwd fields:
    // - claude_session.jsonl has cwd: /Users/test_user/agent-sample
    // - claude_agent.jsonl has cwd: /Users/test_user/agent-sample
    // So they'll both be indexed under the same project

    // Use the actual project paths from the sample files
    let project_sample = "/Users/test_user/agent-sample";

    fixture
        .copy_sample_file_to_project("claude_session.jsonl", "session1.jsonl", project_sample)
        .expect("Failed to copy session1");

    fixture
        .copy_sample_file_to_project("claude_agent.jsonl", "session2.jsonl", project_sample)
        .expect("Failed to copy session2");

    // Index all projects
    fixture.index_update().expect("Failed to index");

    // List all projects - should see sessions
    let mut cmd = fixture.command();
    let output = cmd
        .arg("session")
        .arg("list")
        .arg("--format")
        .arg("json")
        .arg("--all-projects")
        .arg("--no-auto-refresh")
        .output()
        .expect("Failed to run session list --all-projects");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).expect("Parse failed");
    let all_sessions = result["content"]["sessions"]
        .as_array()
        .expect("Expected sessions array");

    assert!(
        !all_sessions.is_empty(),
        "Should have sessions across all projects"
    );

    // Calculate project hash from the actual cwd in sample files
    use agtrace_types::project_hash_from_root;
    let project_sample_hash = project_hash_from_root(project_sample);

    // List with specific project hash
    let mut cmd = fixture.command();
    let output = cmd
        .arg("session")
        .arg("list")
        .arg("--format")
        .arg("json")
        .arg("--project-hash")
        .arg(&project_sample_hash)
        .arg("--no-auto-refresh")
        .output()
        .expect("Failed to run session list with project hash");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).expect("Parse failed");
    let filtered_sessions = result["content"]["sessions"]
        .as_array()
        .expect("Expected sessions array");

    assert!(
        !filtered_sessions.is_empty(),
        "Should have sessions for the project"
    );

    // Verify the sessions belong to the correct project
    for session in filtered_sessions {
        let session_project = session["project_hash"]
            .as_str()
            .expect("Session should have project_hash");
        assert_eq!(
            session_project, project_sample_hash,
            "Session should belong to the correct project"
        );
    }
}

/// Test: Current project has no sessions, other projects have sessions
#[test]
fn test_project_isolation_empty_current_project() {
    let fixture = TestFixture::new();

    fixture
        .setup_provider("claude_code")
        .expect("Failed to setup provider");

    // Create sessions only for project B
    let project_b = "/Users/test_user/project-b";

    fixture
        .copy_sample_file_to_project("claude_session.jsonl", "session1.jsonl", project_b)
        .expect("Failed to copy to project B");

    fixture.index_update().expect("Failed to index");

    // Calculate hash for a different project (project C - which has no sessions)
    use agtrace_types::project_hash_from_root;
    let project_c = "/Users/test_user/project-c";
    let project_c_hash = project_hash_from_root(project_c);

    // List sessions for project C (which has no sessions)
    let mut cmd = fixture.command();
    let output = cmd
        .arg("session")
        .arg("list")
        .arg("--format")
        .arg("json")
        .arg("--project-hash")
        .arg(&project_c_hash)
        .arg("--no-auto-refresh")
        .output()
        .expect("Failed to run session list");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).expect("Parse failed");
    let sessions = result["content"]["sessions"]
        .as_array()
        .expect("Expected sessions array");

    assert_eq!(
        sessions.len(),
        0,
        "Project C should have no sessions (but project B has 1)"
    );

    // Verify --all-projects shows the session from project B
    let mut cmd = fixture.command();
    let output = cmd
        .arg("session")
        .arg("list")
        .arg("--format")
        .arg("json")
        .arg("--all-projects")
        .arg("--no-auto-refresh")
        .output()
        .expect("Failed to run session list --all-projects");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).expect("Parse failed");
    let all_sessions = result["content"]["sessions"]
        .as_array()
        .expect("Expected sessions array");

    assert_eq!(
        all_sessions.len(),
        1,
        "Should have 1 session in project B when viewing all projects"
    );
}

/// Test: Multiple sessions in same project, verify count is correct
#[test]
fn test_project_isolation_multiple_sessions_same_project() {
    let fixture = TestFixture::new();

    fixture
        .setup_provider("claude_code")
        .expect("Failed to setup provider");

    // Use the actual project path from the sample files
    let project_sample = "/Users/test_user/agent-sample";

    // Add multiple session files to the same project directory
    fixture
        .copy_sample_file_to_project("claude_session.jsonl", "session1.jsonl", project_sample)
        .expect("Failed to copy session 1");

    fixture
        .copy_sample_file_to_project("claude_agent.jsonl", "session2.jsonl", project_sample)
        .expect("Failed to copy session 2");

    fixture.index_update().expect("Failed to index");

    use agtrace_types::project_hash_from_root;
    let project_sample_hash = project_hash_from_root(project_sample);

    let mut cmd = fixture.command();
    let output = cmd
        .arg("session")
        .arg("list")
        .arg("--format")
        .arg("json")
        .arg("--project-hash")
        .arg(&project_sample_hash)
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
        "Should have sessions in the project, got {}",
        sessions.len()
    );

    // Verify all sessions belong to the correct project
    for session in sessions {
        let session_project = session["project_hash"]
            .as_str()
            .expect("Session should have project_hash");
        assert_eq!(
            session_project, project_sample_hash,
            "All sessions should belong to the correct project"
        );
    }
}

/// Test: Project list shows projects correctly
#[test]
fn test_project_list_shows_multiple_projects() {
    let fixture = TestFixture::new();

    fixture
        .setup_provider("claude_code")
        .expect("Failed to setup provider");

    // Use the actual project path from the sample files
    let project_sample = "/Users/test_user/agent-sample";

    fixture
        .copy_sample_file_to_project("claude_session.jsonl", "session1.jsonl", project_sample)
        .expect("Failed to copy session");

    fixture.index_update().expect("Failed to index");

    // List all projects
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
        "project list failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).expect("Parse failed");
    let projects = result["content"]["projects"]
        .as_array()
        .expect("Expected projects array");

    assert!(
        !projects.is_empty(),
        "Should have at least 1 project indexed, got {}",
        projects.len()
    );

    use agtrace_types::project_hash_from_root;
    let project_sample_hash = project_hash_from_root(project_sample);

    let project_hashes: Vec<String> = projects
        .iter()
        .filter_map(|p| p["hash"].as_str().map(String::from))
        .collect();

    assert!(
        project_hashes.contains(&project_sample_hash),
        "Should include the project from sample files"
    );
}
