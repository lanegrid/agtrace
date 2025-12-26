//! Multi-provider integration tests.
//!
//! These tests verify that the CLI correctly:
//! 1. Reads provider configuration from config.toml
//! 2. Routes to the correct log directories
//! 3. Selects the appropriate provider adapter
//! 4. Aggregates data from multiple providers

use agtrace_testing::providers::TestProvider;
use agtrace_testing::{TestWorld, assertions};

#[test]
fn test_single_provider_setup() -> anyhow::Result<()> {
    // Test that enabling a single provider works correctly
    let world = TestWorld::new();

    // Enable Claude provider
    world.enable_provider(TestProvider::Claude)?;

    // Add a session
    world.add_session(TestProvider::Claude, "session1.jsonl")?;

    // Index
    let result = world.run(&["index", "update", "--all-projects"])?;
    assert!(result.success(), "Index update failed: {}", result.stderr());

    // List sessions
    let result = world.run(&["session", "list", "--format", "json", "--all-projects"])?;
    assert!(result.success(), "List failed: {}", result.stderr());

    let json = result.json()?;

    // Should have 1 session
    assertions::assert_session_count(&json, 1)?;

    // Should be from Claude
    assertions::assert_session_provider(&json, 0, "claude_code")?;

    Ok(())
}

#[test]
fn test_provider_configuration_routing() -> anyhow::Result<()> {
    // Test that CLI reads provider config and routes to correct directories
    let world = TestWorld::new();

    // Enable Claude provider
    world.enable_provider(TestProvider::Claude)?;

    // Verify provider list shows Claude as enabled
    let result = world.run(&["provider", "list", "--format", "json"])?;
    assert!(result.success());

    let json = result.json()?;

    // Verify Claude is in the provider list
    let providers = json["content"]["providers"]
        .as_array()
        .expect("Expected providers array");

    let claude_provider = providers
        .iter()
        .find(|p| p["name"].as_str() == Some("claude_code"))
        .expect("Claude provider should be listed");

    assert_eq!(claude_provider["enabled"].as_bool(), Some(true));

    Ok(())
}

#[test]
fn test_project_scoped_sessions() -> anyhow::Result<()> {
    // Test that sessions are properly scoped to projects
    let mut world = TestWorld::new()
        .with_project("project-a")
        .with_project("project-b");

    world.enable_provider(TestProvider::Claude)?;

    // Add sessions to different projects
    world.set_cwd("project-a");
    world.add_session(TestProvider::Claude, "session_a.jsonl")?;

    world.set_cwd("project-b");
    world.add_session(TestProvider::Claude, "session_b.jsonl")?;

    // Index
    world.run(&["index", "update", "--all-projects"])?;

    // List all sessions
    let result = world.run(&["session", "list", "--format", "json", "--all-projects"])?;
    assert!(result.success());

    let json = result.json()?;
    assertions::assert_session_count(&json, 2)?;

    // Both should be from Claude
    assertions::assert_all_sessions_from_provider(&json, "claude_code")?;

    // List sessions in project-a only (using project hash)
    use agtrace_types::project_hash_from_root;
    let project_a_path = world.temp_dir().join("project-a");
    let project_a_hash = project_hash_from_root(project_a_path.to_str().unwrap());

    let result = world.run(&[
        "session",
        "list",
        "--format",
        "json",
        "--project-hash",
        &project_a_hash,
        "--no-auto-refresh",
    ])?;
    let json_a = result.json()?;

    // Should have 1 session in project-a
    assertions::assert_session_count(&json_a, 1)?;

    Ok(())
}

// Note: This test is commented out because it requires Gemini sample files.
// Uncomment and adapt when Gemini provider support is fully implemented.
//
// #[test]
// fn test_multi_provider_indexing() -> anyhow::Result<()> {
//     let world = TestWorld::new();
//
//     // Enable multiple providers
//     world.enable_provider(TestProvider::Claude)?;
//     world.enable_provider(TestProvider::Gemini)?;
//
//     // Add sessions from different providers
//     world.add_session(TestProvider::Claude, "claude_session.jsonl")?;
//     world.add_session(TestProvider::Gemini, "gemini_session.jsonl")?;
//
//     // Index all
//     let result = world.run(&["index", "update", "--all-projects"])?;
//     assert!(result.success());
//
//     // List all sessions
//     let result = world.run(&["session", "list", "--format", "json", "--all-projects"])?;
//     assert!(result.success());
//
//     let json = result.json()?;
//     assertions::assert_session_count(&json, 2)?;
//
//     // Filter by provider
//     let result = world.run(&[
//         "session", "list",
//         "--source", "claude_code",
//         "--format", "json",
//         "--all-projects"
//     ])?;
//     let json_claude = result.json()?;
//     assertions::assert_session_count(&json_claude, 1)?;
//     assertions::assert_all_sessions_from_provider(&json_claude, "claude_code")?;
//
//     Ok(())
// }
