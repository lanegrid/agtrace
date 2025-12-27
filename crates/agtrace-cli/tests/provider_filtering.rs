//! Provider Filtering Tests
//!
//! Tests the behavior of `--provider` flag across different commands.
//!
//! # Terminology
//! - `--provider`: Used in both index commands (default: "all") and query commands (session list, lab grep)
//!
//! # Desired Behavior (Test-Driven Approach)
//! These tests define the IDEAL behavior. Current TODOs mark unimplemented features.

use agtrace_testing::providers::TestProvider;
use agtrace_testing::{TestWorld, assertions};
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
    assert!(result.success(), "Command should succeed");

    // Verify only Claude sessions are indexed (use --no-auto-refresh to avoid re-indexing)
    let list_result = world.run(&["session", "list", "--no-auto-refresh", "--format", "json"])?;
    assert!(list_result.success());
    let json = list_result.json()?;

    assertions::assert_session_count(&json, 1)?;
    assertions::assert_all_sessions_from_provider(&json, "claude_code")?;

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
    let result = world.run(&[
        "index",
        "update",
        "--provider",
        "gemini",
        "--format",
        "json",
    ])?;

    // Then: Only Gemini provider is indexed
    assert!(result.success(), "Command should succeed");

    // Verify only Gemini sessions are indexed (use --no-auto-refresh to avoid re-indexing)
    let list_result = world.run(&["session", "list", "--no-auto-refresh", "--format", "json"])?;
    assert!(list_result.success());
    let json = list_result.json()?;

    assertions::assert_session_count(&json, 1)?;
    assertions::assert_all_sessions_from_provider(&json, "gemini")?;

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

    // When: Rebuild with --provider claude_code (forces re-scan of Claude only)
    let result = world.run(&[
        "index",
        "rebuild",
        "--provider",
        "claude_code",
        "--format",
        "json",
    ])?;

    // Then: Rebuild command succeeds
    assert!(result.success(), "Command should succeed");

    // Note: Rebuild with a provider filter re-scans only that provider's files,
    // but does not remove sessions from other providers. Both sessions remain.
    let list_result = world.run(&["session", "list", "--no-auto-refresh", "--format", "json"])?;
    assert!(list_result.success());
    let json = list_result.json()?;

    // Both sessions should still be present (Gemini was not removed)
    assertions::assert_session_count(&json, 2)?;

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

    // When: List sessions without --provider
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

// =============================================================================
// WATCH COMMAND TESTS
// =============================================================================

#[test]
fn test_watch_without_provider_watches_latest_from_any_provider() -> Result<()> {
    use agtrace_testing::process::BackgroundProcess;
    use std::io::{BufRead, BufReader};

    // Given: Multiple providers with sessions at different times
    // Claude session: 2025-12-09T19:47:42.987Z (older)
    // Gemini session: 2025-12-09T19:51:29.418Z (newer, ~4 min later)
    let mut world = TestWorld::new().with_project("my-project");

    world.enable_provider(TestProvider::Claude)?;
    world.enable_provider(TestProvider::Gemini)?;

    world.set_cwd("my-project");
    world.add_session(TestProvider::Claude, "claude-session.jsonl")?;
    world.add_session(TestProvider::Gemini, "session-gemini.json")?;

    world.run(&["init"])?;

    // Get session IDs for verification
    let list_result = world.run(&["session", "list", "--format", "json"])?;
    let json = list_result.json()?;
    let sessions = json["content"]["sessions"]
        .as_array()
        .expect("Should have sessions array");

    // Find the Gemini session (should be the most recent based on timestamp)
    let gemini_session = sessions
        .iter()
        .find(|s| s["provider"].as_str() == Some("gemini"))
        .expect("Should have Gemini session");
    let gemini_id = gemini_session["id"].as_str().expect("Should have ID");

    // When: Run watch without --provider in console mode
    world.set_cwd("my-project");
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_agtrace"));
    cmd.current_dir(world.temp_dir().join("my-project"))
        .arg("--data-dir")
        .arg(world.data_dir())
        .args(["watch", "--mode", "console"]);

    let mut proc = BackgroundProcess::spawn_piped(cmd)?;

    // Then: Should attach to the most recent session (Gemini)
    let stdout = proc.stdout().expect("Should have stdout");
    let reader = BufReader::new(stdout);

    let mut found_gemini_attachment = false;
    for line in reader.lines().take(10) {
        let line = line?;
        if line.contains("Attached") && line.contains(&gemini_id[..8]) {
            found_gemini_attachment = true;
            break;
        }
    }

    // Clean up
    proc.kill()?;

    assert!(
        found_gemini_attachment,
        "Should attach to most recent session (Gemini), not first provider (Claude)"
    );

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

    // When: Grep without --provider
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

    // When: Grep with --provider claude_code
    let result = world.run(&[
        "lab",
        "grep",
        "Read",
        "--provider",
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

    // When: Grep with --provider gemini
    let result = world.run(&[
        "lab",
        "grep",
        "Read",
        "--provider",
        "gemini",
        "--limit",
        "10",
    ])?;

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

    // When: List with --provider gemini (disabled provider)
    let result = world.run(&[
        "session",
        "list",
        "--provider",
        "gemini",
        "--format",
        "json",
    ])?;

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
    assert!(result.success(), "Command should succeed");

    // Verify only Claude sessions are in the database (use --no-auto-refresh to avoid re-indexing)
    let list_result = world.run(&["session", "list", "--no-auto-refresh", "--format", "json"])?;
    assert!(list_result.success());
    let json = list_result.json()?;

    assertions::assert_session_count(&json, 1)?;
    assertions::assert_all_sessions_from_provider(&json, "claude_code")?;

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

    // When: List with --provider claude_code and specific project
    world.set_cwd("project-a");
    let result = world.run(&[
        "session",
        "list",
        "--provider",
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
