//! List & Filtering Tests
//!
//! Verifies that session list filtering works correctly
//! with --provider, --all-projects, and other filters.

use agtrace_testing::providers::TestProvider;
use agtrace_testing::{TestWorld, assertions};
use anyhow::Result;

#[test]
fn test_list_filter_by_source_provider() -> Result<()> {
    // Given: Multiple providers (Claude and Gemini) with sessions
    let mut world = TestWorld::new().with_project("my-project");

    world.enable_provider(TestProvider::Claude)?;
    world.enable_provider(TestProvider::Gemini)?;

    world.set_cwd("my-project");
    world.add_session(TestProvider::Claude, "claude-session.jsonl")?;
    world.add_session(TestProvider::Gemini, "session-gemini.json")?;

    world.run(&["init"])?;

    // When: List with --provider claude_code
    let result = world.run(&[
        "session",
        "list",
        "--provider",
        "claude_code",
        "--format",
        "json",
    ])?;

    // Then: Only Claude sessions are shown
    assert!(result.success(), "Command should succeed");
    let json = result.json()?;
    assertions::assert_session_count(&json, 1)?;
    assertions::assert_all_sessions_from_provider(&json, "claude_code")?;

    // When: List with --provider gemini
    let result = world.run(&[
        "session",
        "list",
        "--provider",
        "gemini",
        "--format",
        "json",
    ])?;

    // Then: Only Gemini sessions are shown
    assert!(result.success(), "Command should succeed");
    let json = result.json()?;
    assertions::assert_session_count(&json, 1)?;
    assertions::assert_all_sessions_from_provider(&json, "gemini")?;

    Ok(())
}

#[test]
fn test_list_all_projects_shows_sessions_from_all_projects() -> Result<()> {
    // Given: Multiple projects with sessions
    let mut world = TestWorld::new()
        .with_project("project-a")
        .with_project("project-b")
        .with_project("project-c");

    world.enable_provider(TestProvider::Claude)?;

    world.set_cwd("project-a");
    world.add_session(TestProvider::Claude, "session-a.jsonl")?;

    world.set_cwd("project-b");
    world.add_session(TestProvider::Claude, "session-b.jsonl")?;

    world.set_cwd("project-c");
    world.add_session(TestProvider::Claude, "session-c.jsonl")?;

    world.run(&["init", "--all-projects"])?;

    // When: List with --all-projects from any directory
    world.set_cwd("project-a");
    let result = world.run(&["session", "list", "--all-projects", "--format", "json"])?;

    // Then: All projects' sessions are shown
    assert!(result.success(), "Command should succeed");
    let json = result.json()?;
    assertions::assert_session_count(&json, 3)?;

    Ok(())
}

#[test]
fn test_list_without_all_projects_shows_only_current_project() -> Result<()> {
    // Given: Multiple projects with sessions
    let mut world = TestWorld::new()
        .with_project("project-a")
        .with_project("project-b");

    world.enable_provider(TestProvider::Claude)?;

    world.set_cwd("project-a");
    world.add_session(TestProvider::Claude, "session-a1.jsonl")?;
    world.add_session(TestProvider::Claude, "session-a2.jsonl")?;

    world.set_cwd("project-b");
    world.add_session(TestProvider::Claude, "session-b1.jsonl")?;

    world.run(&["init", "--all-projects"])?;

    // When: List without --all-projects from project-a
    world.set_cwd("project-a");
    let result = world.run(&["session", "list", "--format", "json"])?;

    // Then: Only project-a's sessions are shown
    assert!(result.success(), "Command should succeed");
    let json = result.json()?;
    assertions::assert_session_count(&json, 2)?;

    let project_a_path = world.temp_dir().join("project-a");
    let expected_hash =
        agtrace_sdk::utils::project_hash_from_root(&project_a_path.to_string_lossy());
    assertions::assert_sessions_belong_to_project(&json, expected_hash.as_str())?;

    Ok(())
}

#[test]
fn test_list_limit_parameter() -> Result<()> {
    // Given: Project with many sessions
    let mut world = TestWorld::new().with_project("my-project");

    world.enable_provider(TestProvider::Claude)?;
    world.set_cwd("my-project");

    // Add multiple sessions
    for i in 1..=5 {
        world.add_session(TestProvider::Claude, &format!("session-{}.jsonl", i))?;
    }

    world.run(&["init"])?;

    // When: List with --limit 3
    let result = world.run(&["session", "list", "--limit", "3", "--format", "json"])?;

    // Then: Only 3 sessions are shown
    assert!(result.success(), "Command should succeed");
    let json = result.json()?;
    assertions::assert_session_count(&json, 3)?;

    Ok(())
}

#[test]
fn test_list_combined_filters() -> Result<()> {
    // Given: Multiple projects and providers
    let mut world = TestWorld::new()
        .with_project("project-a")
        .with_project("project-b");

    world.enable_provider(TestProvider::Claude)?;
    world.enable_provider(TestProvider::Gemini)?;

    // Project A: Claude sessions
    world.set_cwd("project-a");
    world.add_session(TestProvider::Claude, "claude-a1.jsonl")?;
    world.add_session(TestProvider::Claude, "claude-a2.jsonl")?;

    // Project B: Mixed sessions
    world.set_cwd("project-b");
    world.add_session(TestProvider::Claude, "claude-b1.jsonl")?;
    world.add_session(TestProvider::Gemini, "session-b1.json")?;

    world.run(&["init", "--all-projects"])?;

    // When: List with --provider claude_code and --all-projects
    world.set_cwd("project-a");
    let result = world.run(&[
        "session",
        "list",
        "--provider",
        "claude_code",
        "--all-projects",
        "--format",
        "json",
    ])?;

    // Then: Only Claude sessions from all projects
    assert!(result.success(), "Command should succeed");
    let json = result.json()?;
    assertions::assert_session_count(&json, 3)?;
    assertions::assert_all_sessions_from_provider(&json, "claude_code")?;

    Ok(())
}

#[test]
fn test_list_no_sessions_returns_empty_array() -> Result<()> {
    // Given: Initialized project with no sessions
    let mut world = TestWorld::new().with_project("empty-project");

    world.enable_provider(TestProvider::Claude)?;
    world.set_cwd("empty-project");

    world.run(&["init"])?;

    // When: List sessions
    let result = world.run(&["session", "list", "--format", "json"])?;

    // Then: Empty array is returned
    assert!(result.success(), "Command should succeed");
    let json = result.json()?;
    assertions::assert_session_count(&json, 0)?;

    Ok(())
}
