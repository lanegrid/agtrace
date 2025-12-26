//! Example test demonstrating the TestWorld pattern.
//!
//! This test shows how to use the new agtrace-testing infrastructure
//! to write more maintainable and robust integration tests.

use agtrace_testing::{TestWorld, assertions};
use assert_cmd::cargo::cargo_bin_cmd;

#[test]
fn test_testworld_project_isolation() {
    // Create a new test environment
    let world = TestWorld::new();

    // Setup provider
    let mut cmd = cargo_bin_cmd!("agtrace");
    world
        .configure_command(&mut cmd)
        .arg("provider")
        .arg("set")
        .arg("claude_code")
        .arg("--log-root")
        .arg(world.log_root())
        .arg("--enable");
    let output = cmd.output().expect("Failed to setup provider");
    assert!(output.status.success());

    // Create sessions for two different projects
    let project_a = "/Users/test_user/project-a";
    let project_b = "/Users/test_user/project-b";

    world
        .copy_sample_to_project_with_cwd("claude_session.jsonl", "session1.jsonl", project_a)
        .expect("Failed to copy to project A");

    world
        .copy_sample_to_project_with_cwd("claude_agent.jsonl", "session2.jsonl", project_b)
        .expect("Failed to copy to project B");

    // Index all projects
    let mut cmd = cargo_bin_cmd!("agtrace");
    world
        .configure_command(&mut cmd)
        .arg("index")
        .arg("update")
        .arg("--all-projects")
        .arg("--verbose");
    let output = cmd.output().expect("Failed to index");
    assert!(output.status.success());

    // List all projects - should see 2 sessions across 2 projects
    let mut cmd = cargo_bin_cmd!("agtrace");
    world
        .configure_command(&mut cmd)
        .arg("session")
        .arg("list")
        .arg("--format")
        .arg("json")
        .arg("--all-projects")
        .arg("--no-auto-refresh");
    let output = cmd.output().expect("Failed to run session list");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Parse failed");

    // Use custom assertions from agtrace-testing
    assertions::assert_session_count(&json, 2).expect("Should have 2 sessions");

    // Verify sessions belong to different projects
    let sessions = json["content"]["sessions"].as_array().unwrap();
    let project_hashes: Vec<&str> = sessions
        .iter()
        .map(|s| s["project_hash"].as_str().unwrap())
        .collect();

    // Two sessions should have different project hashes
    use agtrace_types::project_hash_from_root;
    let project_a_hash = project_hash_from_root(project_a);
    let project_b_hash = project_hash_from_root(project_b);

    assert!(project_hashes.contains(&project_a_hash.as_str()));
    assert!(project_hashes.contains(&project_b_hash.as_str()));
}

#[test]
fn test_testworld_cwd_change() {
    // Demonstrate changing working directory (builder pattern)
    let world = TestWorld::new()
        .with_project("my-project")
        .enter_dir("my-project");

    assert!(world.cwd().ends_with("my-project"));
}

#[test]
fn test_testworld_set_cwd_multiple_times() {
    // Demonstrate mutable directory changes
    let mut world = TestWorld::new()
        .with_project("project-a")
        .with_project("project-b");

    // Initial cwd is temp root (not in any project)
    let initial_cwd = world.cwd().to_path_buf();
    assert!(!initial_cwd.ends_with("project-a"));
    assert!(!initial_cwd.ends_with("project-b"));

    // Move to project-a
    world.set_cwd("project-a");
    assert!(world.cwd().ends_with("project-a"));

    // Move to project-b
    world.set_cwd("project-b");
    assert!(world.cwd().ends_with("project-b"));

    // Move back to temp root
    let temp_dir = world.temp_dir().to_path_buf();
    world.set_cwd(temp_dir);
    assert!(!world.cwd().ends_with("project-b"));
}

#[test]
fn test_testworld_run_in_dir() {
    // Demonstrate temporary directory context
    let mut world = TestWorld::new()
        .with_project("project-a")
        .with_project("project-b");

    // Setup provider
    world
        .run(&[
            "provider",
            "set",
            "claude_code",
            "--log-root",
            world.log_root().to_str().unwrap(),
            "--enable",
        ])
        .expect("Failed to setup provider");

    // Copy samples to different projects
    world
        .copy_sample_to_project_with_cwd(
            "claude_session.jsonl",
            "session_a.jsonl",
            "/Users/test_user/project-a",
        )
        .expect("Failed to copy to project A");

    world
        .copy_sample_to_project_with_cwd(
            "claude_agent.jsonl",
            "session_b.jsonl",
            "/Users/test_user/project-b",
        )
        .expect("Failed to copy to project B");

    // Index all
    world
        .run(&["index", "update", "--all-projects"])
        .expect("Failed to index");

    // Run in project-a without permanently changing cwd
    let result_a = world
        .run_in_dir(&["session", "list", "--format", "json"], "project-a")
        .expect("Failed to list in project-a");

    assert!(result_a.success());
    let json_a = result_a.json().expect("Failed to parse JSON");

    // Run in project-b - cwd is still at original location
    let result_b = world
        .run_in_dir(&["session", "list", "--format", "json"], "project-b")
        .expect("Failed to list in project-b");

    assert!(result_b.success());
    let json_b = result_b.json().expect("Failed to parse JSON");

    // Verify sessions belong to different projects
    use agtrace_types::project_hash_from_root;
    let hash_a = project_hash_from_root("/Users/test_user/project-a");
    let hash_b = project_hash_from_root("/Users/test_user/project-b");

    assertions::assert_sessions_belong_to_project(&json_a, &hash_a)
        .expect("Sessions should belong to project A");
    assertions::assert_sessions_belong_to_project(&json_b, &hash_b)
        .expect("Sessions should belong to project B");
}

#[test]
fn test_testworld_run_convenience_method() {
    // Demonstrate the run() convenience method
    let world = TestWorld::new();

    // Setup provider using run()
    let result = world
        .run(&[
            "provider",
            "set",
            "claude_code",
            "--log-root",
            world.log_root().to_str().unwrap(),
            "--enable",
        ])
        .expect("Failed to setup provider");

    assert!(
        result.success(),
        "Provider setup failed: {}",
        result.stderr()
    );

    // Copy sample data
    world
        .copy_sample_to_project_with_cwd(
            "claude_session.jsonl",
            "session1.jsonl",
            "/Users/test_user/project-a",
        )
        .expect("Failed to copy sample");

    // Index using run()
    let result = world
        .run(&["index", "update", "--all-projects", "--verbose"])
        .expect("Failed to index");

    assert!(result.success(), "Index failed: {}", result.stderr());

    // List sessions using run()
    let result = world
        .run(&["session", "list", "--format", "json", "--all-projects"])
        .expect("Failed to list sessions");

    assert!(result.success(), "List failed: {}", result.stderr());

    // Verify using custom assertions
    let json = result.json().expect("Failed to parse JSON");
    assertions::assert_session_count(&json, 1).expect("Should have 1 session");
}
