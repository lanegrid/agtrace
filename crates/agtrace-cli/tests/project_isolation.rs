//! Project Isolation Tests
//!
//! Verifies that sessions are correctly isolated by project hash,
//! preventing cross-project data leakage.

use agtrace_testing::providers::TestProvider;
use agtrace_testing::{TestWorld, assertions};
use anyhow::Result;

#[test]
fn test_isolation_project_a_and_b_list_shows_only_current_project() -> Result<()> {
    // Given: Project A and B both have sessions
    let mut world = TestWorld::new()
        .with_project("project-a")
        .with_project("project-b");

    world.enable_provider(TestProvider::Claude)?;

    // Add sessions to both projects
    world.set_cwd("project-a");
    world.add_session(TestProvider::Claude, "session-a1.jsonl")?;

    world.set_cwd("project-b");
    world.add_session(TestProvider::Claude, "session-b1.jsonl")?;

    // Index both projects
    world.run(&["init"])?;

    // When: List sessions in project A
    world.set_cwd("project-a");
    let result = world.run(&["session", "list", "--format", "json"])?;

    // Then: Only project A's sessions are shown
    assert!(result.success(), "Command should succeed");
    let json = result.json()?;
    assertions::assert_session_count(&json, 1)?;

    // Verify project hash matches project A
    let project_a_path = world.temp_dir().join("project-a");
    let expected_hash =
        agtrace_sdk::utils::project_hash_from_root(&project_a_path.to_string_lossy());
    assertions::assert_sessions_belong_to_project(&json, expected_hash.as_str())?;

    Ok(())
}

#[test]
fn test_isolation_empty_project_shows_zero_results() -> Result<()> {
    // Given: Project A has data, Project B is empty
    let mut world = TestWorld::new()
        .with_project("project-a")
        .with_project("project-b");

    world.enable_provider(TestProvider::Claude)?;

    // Add session only to project A
    world.set_cwd("project-a");
    world.add_session(TestProvider::Claude, "session-a1.jsonl")?;

    world.run(&["init"])?;

    // When: List sessions in project B (empty)
    world.set_cwd("project-b");
    let result = world.run(&["session", "list", "--format", "json"])?;

    // Then: 0 results (no leak from project A)
    assert!(result.success(), "Command should succeed");
    let json = result.json()?;
    assertions::assert_session_count(&json, 0)?;

    Ok(())
}

#[test]
fn test_isolation_multiple_sessions_in_same_project() -> Result<()> {
    // Given: Multiple session files in the same project
    let mut world = TestWorld::new().with_project("my-project");

    world.enable_provider(TestProvider::Claude)?;

    world.set_cwd("my-project");
    world.add_session(TestProvider::Claude, "session1.jsonl")?;
    world.add_session(TestProvider::Claude, "session2.jsonl")?;
    world.add_session(TestProvider::Claude, "session3.jsonl")?;

    world.run(&["init"])?;

    // When: List sessions
    let result = world.run(&["session", "list", "--format", "json"])?;

    // Then: All sessions belong to this project
    assert!(result.success(), "Command should succeed");
    let json = result.json()?;
    assertions::assert_session_count(&json, 3)?;

    let project_path = world.temp_dir().join("my-project");
    let expected_hash = agtrace_sdk::utils::project_hash_from_root(&project_path.to_string_lossy());
    assertions::assert_sessions_belong_to_project(&json, expected_hash.as_str())?;

    Ok(())
}

#[test]
fn test_isolation_all_projects_flag_shows_all_sessions() -> Result<()> {
    // Given: Multiple projects with sessions
    let mut world = TestWorld::new()
        .with_project("project-a")
        .with_project("project-b");

    world.enable_provider(TestProvider::Claude)?;

    world.set_cwd("project-a");
    world.add_session(TestProvider::Claude, "session-a.jsonl")?;

    world.set_cwd("project-b");
    world.add_session(TestProvider::Claude, "session-b.jsonl")?;

    world.run(&["init", "--all-projects"])?;

    // When: List with --all-projects from project A
    world.set_cwd("project-a");
    let result = world.run(&["session", "list", "--all-projects", "--format", "json"])?;

    // Then: Both projects' sessions are shown
    assert!(result.success(), "Command should succeed");
    let json = result.json()?;
    assertions::assert_session_count(&json, 2)?;

    Ok(())
}
