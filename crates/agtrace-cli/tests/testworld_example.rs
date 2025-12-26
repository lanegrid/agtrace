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
    // Demonstrate changing working directory
    let world = TestWorld::new()
        .with_project("my-project")
        .enter_dir("my-project");

    assert!(world.cwd().ends_with("my-project"));
}
