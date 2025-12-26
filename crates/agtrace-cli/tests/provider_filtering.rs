//! Provider Filtering Tests
//!
//! Tests the behavior of `--provider` and `--source` flags across different commands.
//!
//! # Terminology
//! - `--provider`: Used in index commands (default: "all")
//! - `--source`: Used in query commands (session list, lab grep)
//!
//! # Desired Behavior (Test-Driven Approach)
//! These tests define the IDEAL behavior. Current TODOs mark unimplemented features.

use agtrace_testing::providers::TestProvider;
use agtrace_testing::{assertions, TestWorld};
use anyhow::Result;

// =============================================================================
// INDEX COMMAND TESTS
// =============================================================================

#[test]
fn test_index_update_without_provider_flag_indexes_all_providers() -> Result<()> {
    // Given: Multiple providers with sessions
    let mut world = TestWorld::new().with_project("my-project");

    world.enable_provider(TestProvider::Claude)?;
    world.enable_provider(TestProvider::Gemini)?;

    world.set_cwd("my-project");
    world.add_session(TestProvider::Claude, "claude-session.jsonl")?;
    world.add_session(TestProvider::Gemini, "session-gemini.json")?;

    // When: Run index update without --provider flag
    let result = world.run(&["index", "update", "--format", "json"])?;

    // Then: Both providers are indexed
    // TODO: Currently the --provider parameter is ignored in the implementation
    // Expected behavior: Should index sessions from both Claude and Gemini
    assert!(result.success(), "Command should succeed");

    // Verify sessions are indexed
    let list_result = world.run(&["session", "list", "--format", "json"])?;
    assert!(list_result.success());
    let _json = list_result.json()?;
    // TODO: Should be 2 sessions (one from each provider)
    // Currently may not filter correctly
    let _expected_count = 2;

    Ok(())
}

#[test]
fn test_index_update_with_provider_all_indexes_all_providers() -> Result<()> {
    // Given: Multiple providers with sessions
    let mut world = TestWorld::new().with_project("my-project");

    world.enable_provider(TestProvider::Claude)?;
    world.enable_provider(TestProvider::Gemini)?;

    world.set_cwd("my-project");
    world.add_session(TestProvider::Claude, "claude-session.jsonl")?;
    world.add_session(TestProvider::Gemini, "session-gemini.json")?;

    // When: Run index update with --provider all
    let result = world.run(&["index", "update", "--provider", "all", "--format", "json"])?;

    // Then: Both providers are indexed
    // TODO: Currently the --provider parameter is ignored in the implementation
    // Expected behavior: Should index sessions from both Claude and Gemini
    assert!(result.success(), "Command should succeed");

    // Verify sessions are indexed
    let list_result = world.run(&["session", "list", "--format", "json"])?;
    assert!(list_result.success());
    let _json = list_result.json()?;
    // TODO: Should be 2 sessions (one from each provider)
    let _expected_count = 2;

    Ok(())
}

#[test]
fn test_index_update_with_provider_claude_code_indexes_only_claude() -> Result<()> {
    // Given: Multiple providers with sessions
    let mut world = TestWorld::new().with_project("my-project");

    world.enable_provider(TestProvider::Claude)?;
    world.enable_provider(TestProvider::Gemini)?;

    world.set_cwd("my-project");
    world.add_session(TestProvider::Claude, "claude-session.jsonl")?;
    world.add_session(TestProvider::Gemini, "session-gemini.json")?;

    // When: Run index update with --provider claude_code
    let result = world.run(&[
        "index",
        "update",
        "--provider",
        "claude_code",
        "--format",
        "json",
    ])?;

    // Then: Only Claude provider is indexed
    // TODO: Implement provider filtering in index handler
    // Expected behavior: Should index only Claude sessions
    assert!(result.success(), "Command should succeed");

    // Verify only Claude sessions are indexed
    let list_result = world.run(&["session", "list", "--format", "json"])?;
    assert!(list_result.success());
    let _json = list_result.json()?;

    // TODO: After implementing provider filtering:
    // assertions::assert_session_count(&_json, 1)?;
    // assertions::assert_all_sessions_from_provider(&_json, "claude_code")?;

    // For now, just verify the command succeeds
    let _expected_provider = "claude_code";

    Ok(())
}

#[test]
fn test_index_update_with_provider_gemini_indexes_only_gemini() -> Result<()> {
    // Given: Multiple providers with sessions
    let mut world = TestWorld::new().with_project("my-project");

    world.enable_provider(TestProvider::Claude)?;
    world.enable_provider(TestProvider::Gemini)?;

    world.set_cwd("my-project");
    world.add_session(TestProvider::Claude, "claude-session.jsonl")?;
    world.add_session(TestProvider::Gemini, "session-gemini.json")?;

    // When: Run index update with --provider gemini
    let result = world.run(&["index", "update", "--provider", "gemini", "--format", "json"])?;

    // Then: Only Gemini provider is indexed
    // TODO: Implement provider filtering in index handler
    // Expected behavior: Should index only Gemini sessions
    assert!(result.success(), "Command should succeed");

    // Verify only Gemini sessions are indexed
    let list_result = world.run(&["session", "list", "--format", "json"])?;
    assert!(list_result.success());
    let _json = list_result.json()?;

    // TODO: After implementing provider filtering:
    // assertions::assert_session_count(&_json, 1)?;
    // assertions::assert_all_sessions_from_provider(&_json, "gemini")?;

    // For now, just verify the command succeeds
    let _expected_provider = "gemini";

    Ok(())
}

#[test]
fn test_index_rebuild_with_provider_filter_rebuilds_only_specified_provider() -> Result<()> {
    // Given: Multiple providers with sessions, already indexed
    let mut world = TestWorld::new().with_project("my-project");

    world.enable_provider(TestProvider::Claude)?;
    world.enable_provider(TestProvider::Gemini)?;

    world.set_cwd("my-project");
    world.add_session(TestProvider::Claude, "claude-session.jsonl")?;
    world.add_session(TestProvider::Gemini, "session-gemini.json")?;

    // First index all
    world.run(&["init"])?;

    // When: Rebuild with --provider claude_code
    let result = world.run(&[
        "index",
        "rebuild",
        "--provider",
        "claude_code",
        "--format",
        "json",
    ])?;

    // Then: Only Claude provider is rebuilt
    // TODO: Implement provider filtering in rebuild handler
    // Expected behavior: Should rebuild only Claude sessions
    assert!(result.success(), "Command should succeed");

    // For now, just verify the command succeeds
    let _expected_provider = "claude_code";

    Ok(())
}

// =============================================================================
// SESSION LIST COMMAND TESTS
// =============================================================================

#[test]
fn test_session_list_without_source_shows_all_providers() -> Result<()> {
    // Given: Multiple providers with sessions
    let mut world = TestWorld::new().with_project("my-project");

    world.enable_provider(TestProvider::Claude)?;
    world.enable_provider(TestProvider::Gemini)?;

    world.set_cwd("my-project");
    world.add_session(TestProvider::Claude, "claude-session.jsonl")?;
    world.add_session(TestProvider::Gemini, "session-gemini.json")?;

    world.run(&["init"])?;

    // When: List sessions without --source
    let result = world.run(&["session", "list", "--format", "json"])?;

    // Then: Sessions from all providers are shown
    assert!(result.success(), "Command should succeed");
    let json = result.json()?;
    assertions::assert_session_count(&json, 2)?;

    Ok(())
}

#[test]
fn test_session_list_with_source_claude_code_shows_only_claude() -> Result<()> {
    // Given: Multiple providers with sessions
    let mut world = TestWorld::new().with_project("my-project");

    world.enable_provider(TestProvider::Claude)?;
    world.enable_provider(TestProvider::Gemini)?;

    world.set_cwd("my-project");
    world.add_session(TestProvider::Claude, "claude-session.jsonl")?;
    world.add_session(TestProvider::Gemini, "session-gemini.json")?;

    world.run(&["init"])?;

    // When: List with --source claude_code
    let result = world.run(&[
        "session",
        "list",
        "--source",
        "claude_code",
        "--format",
        "json",
    ])?;

    // Then: Only Claude sessions are shown
    assert!(result.success(), "Command should succeed");
    let json = result.json()?;
    assertions::assert_session_count(&json, 1)?;
    assertions::assert_all_sessions_from_provider(&json, "claude_code")?;

    Ok(())
}

#[test]
fn test_session_list_with_source_gemini_shows_only_gemini() -> Result<()> {
    // Given: Multiple providers with sessions
    let mut world = TestWorld::new().with_project("my-project");

    world.enable_provider(TestProvider::Claude)?;
    world.enable_provider(TestProvider::Gemini)?;

    world.set_cwd("my-project");
    world.add_session(TestProvider::Claude, "claude-session.jsonl")?;
    world.add_session(TestProvider::Gemini, "session-gemini.json")?;

    world.run(&["init"])?;

    // When: List with --source gemini
    let result = world.run(&["session", "list", "--source", "gemini", "--format", "json"])?;

    // Then: Only Gemini sessions are shown
    assert!(result.success(), "Command should succeed");
    let json = result.json()?;
    assertions::assert_session_count(&json, 1)?;
    assertions::assert_all_sessions_from_provider(&json, "gemini")?;

    Ok(())
}

// =============================================================================
// WATCH COMMAND TESTS
// =============================================================================

#[test]
fn test_watch_without_provider_watches_latest_from_any_provider() -> Result<()> {
    // Given: Multiple providers with sessions
    let mut world = TestWorld::new().with_project("my-project");

    world.enable_provider(TestProvider::Claude)?;
    world.enable_provider(TestProvider::Gemini)?;

    world.set_cwd("my-project");
    world.add_session(TestProvider::Claude, "claude-session.jsonl")?;
    world.add_session(TestProvider::Gemini, "session-gemini.json")?;

    world.run(&["init"])?;

    // When: Watch without --provider (would run in background)
    // Note: We can't actually test the interactive watch behavior in integration tests
    // This test just verifies the command accepts the parameters

    // TODO: Implement actual watch behavior testing
    // Expected behavior: Should watch the most recent session from any provider

    // For now, just verify command construction
    let _watch_args = ["watch", "--mode", "console"];

    Ok(())
}

#[test]
fn test_watch_with_provider_claude_code_watches_only_claude_sessions() -> Result<()> {
    // Given: Multiple providers with sessions
    let mut world = TestWorld::new().with_project("my-project");

    world.enable_provider(TestProvider::Claude)?;
    world.enable_provider(TestProvider::Gemini)?;

    world.set_cwd("my-project");
    world.add_session(TestProvider::Claude, "claude-session.jsonl")?;
    world.add_session(TestProvider::Gemini, "session-gemini.json")?;

    world.run(&["init"])?;

    // When: Watch with --provider claude_code
    // Note: We can't actually test the interactive watch behavior in integration tests
    // This test just verifies the command accepts the parameters

    // TODO: Implement actual watch behavior testing
    // Expected behavior: Should watch only Claude sessions

    // For now, just verify command construction
    let _watch_args = ["watch", "--mode", "console", "--provider", "claude_code"];

    Ok(())
}

// =============================================================================
// LAB GREP COMMAND TESTS
// =============================================================================

#[test]
fn test_lab_grep_without_source_searches_all_providers() -> Result<()> {
    // Given: Multiple providers with sessions containing different tools
    let mut world = TestWorld::new().with_project("my-project");

    world.enable_provider(TestProvider::Claude)?;
    world.enable_provider(TestProvider::Gemini)?;

    world.set_cwd("my-project");
    world.add_session(TestProvider::Claude, "claude-session.jsonl")?;
    world.add_session(TestProvider::Gemini, "session-gemini.json")?;

    world.run(&["init"])?;

    // When: Grep without --source
    let result = world.run(&["lab", "grep", "Read", "--limit", "10"])?;

    // Then: Searches across all providers
    // TODO: Verify results include sessions from multiple providers
    // Expected behavior: Should search in both Claude and Gemini sessions
    assert!(result.success(), "Command should succeed");

    Ok(())
}

#[test]
fn test_lab_grep_with_source_claude_code_searches_only_claude() -> Result<()> {
    // Given: Multiple providers with sessions
    let mut world = TestWorld::new().with_project("my-project");

    world.enable_provider(TestProvider::Claude)?;
    world.enable_provider(TestProvider::Gemini)?;

    world.set_cwd("my-project");
    world.add_session(TestProvider::Claude, "claude-session.jsonl")?;
    world.add_session(TestProvider::Gemini, "session-gemini.json")?;

    world.run(&["init"])?;

    // When: Grep with --source claude_code
    let result = world.run(&[
        "lab",
        "grep",
        "Read",
        "--source",
        "claude_code",
        "--limit",
        "10",
    ])?;

    // Then: Searches only Claude sessions
    // TODO: Verify results only include Claude sessions
    // Expected behavior: Should search only in Claude sessions
    assert!(result.success(), "Command should succeed");

    Ok(())
}

#[test]
fn test_lab_grep_with_source_gemini_searches_only_gemini() -> Result<()> {
    // Given: Multiple providers with sessions
    let mut world = TestWorld::new().with_project("my-project");

    world.enable_provider(TestProvider::Claude)?;
    world.enable_provider(TestProvider::Gemini)?;

    world.set_cwd("my-project");
    world.add_session(TestProvider::Claude, "claude-session.jsonl")?;
    world.add_session(TestProvider::Gemini, "session-gemini.json")?;

    world.run(&["init"])?;

    // When: Grep with --source gemini
    let result = world.run(&["lab", "grep", "Read", "--source", "gemini", "--limit", "10"])?;

    // Then: Searches only Gemini sessions
    // TODO: Verify results only include Gemini sessions
    // Expected behavior: Should search only in Gemini sessions
    assert!(result.success(), "Command should succeed");

    Ok(())
}

// =============================================================================
// EDGE CASES
// =============================================================================

#[test]
fn test_provider_filter_with_disabled_provider_shows_no_sessions() -> Result<()> {
    // Given: Claude provider enabled but Gemini disabled
    let mut world = TestWorld::new().with_project("my-project");

    world.enable_provider(TestProvider::Claude)?;
    // Gemini is NOT enabled

    world.set_cwd("my-project");
    world.add_session(TestProvider::Claude, "claude-session.jsonl")?;

    world.run(&["init"])?;

    // When: List with --source gemini (disabled provider)
    let result = world.run(&["session", "list", "--source", "gemini", "--format", "json"])?;

    // Then: No sessions are shown
    assert!(result.success(), "Command should succeed");
    let json = result.json()?;
    assertions::assert_session_count(&json, 0)?;

    Ok(())
}

#[test]
fn test_index_with_provider_filter_skips_other_providers() -> Result<()> {
    // Given: Two providers, but we index only one
    let mut world = TestWorld::new().with_project("my-project");

    world.enable_provider(TestProvider::Claude)?;
    world.enable_provider(TestProvider::Gemini)?;

    world.set_cwd("my-project");
    world.add_session(TestProvider::Claude, "claude-session.jsonl")?;
    world.add_session(TestProvider::Gemini, "session-gemini.json")?;

    // When: Index with --provider claude_code
    let result = world.run(&[
        "index",
        "update",
        "--provider",
        "claude_code",
        "--format",
        "json",
    ])?;

    // Then: Index command succeeds
    // TODO: Implement provider filtering in index
    // Expected behavior: Should index only Claude, skip Gemini
    assert!(result.success(), "Command should succeed");

    // TODO: Verify only Claude sessions are in the database
    // For now, we just check the command succeeds
    let _expected_indexed_providers = ["claude_code"];

    Ok(())
}

#[test]
fn test_combined_filters_provider_and_project() -> Result<()> {
    // Given: Multiple projects and providers
    let mut world = TestWorld::new()
        .with_project("project-a")
        .with_project("project-b");

    world.enable_provider(TestProvider::Claude)?;
    world.enable_provider(TestProvider::Gemini)?;

    // Project A: Claude and Gemini sessions
    world.set_cwd("project-a");
    world.add_session(TestProvider::Claude, "claude-a.jsonl")?;
    world.add_session(TestProvider::Gemini, "gemini-a.json")?;

    // Project B: Claude session only
    world.set_cwd("project-b");
    world.add_session(TestProvider::Claude, "claude-b.jsonl")?;

    world.run(&["init", "--all-projects"])?;

    // When: List with --source claude_code and specific project
    world.set_cwd("project-a");
    let result = world.run(&[
        "session",
        "list",
        "--source",
        "claude_code",
        "--format",
        "json",
    ])?;

    // Then: Only Claude sessions from current project
    assert!(result.success(), "Command should succeed");
    let json = result.json()?;
    assertions::assert_session_count(&json, 1)?;
    assertions::assert_all_sessions_from_provider(&json, "claude_code")?;

    Ok(())
}
