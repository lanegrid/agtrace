//! Spawn Context Tests for Codex Subagent Parent Linking
//!
//! Tests the timestamp correlation logic that links Codex subagent sessions
//! back to their parent CLI sessions via the `entered_review_mode` event.
//!
//! Timestamp Design:
//! - T+0.000s: Parent session starts
//! - T+0.200s: First spawn event (entered_review_mode)
//! - T+0.225s: Subagent-001 session (within 100ms of first spawn -> MATCHES)
//! - T+2.000s: Orphan subagent session (>100ms from any spawn -> NO MATCH)
//! - T+5.300s: Second spawn event (entered_review_mode)
//! - T+5.325s: Subagent-002 session (within 100ms of second spawn -> MATCHES)

use agtrace_sdk::{Client, SessionFilter, SystemClient};
use agtrace_testing::TestWorld;
use agtrace_testing::providers::TestProvider;
use anyhow::Result;

/// Helper function to initialize the workspace with SDK.
async fn initialize_workspace(world: &TestWorld) -> Result<Client> {
    let config = agtrace_sdk::types::InitConfig {
        data_dir: world.data_dir().to_path_buf(),
        project_root: Some(world.cwd().to_path_buf()),
        all_projects: true,
        refresh: false,
    };
    SystemClient::initialize(config, None::<fn(agtrace_sdk::types::InitProgress)>)?;
    world.get_client().await
}

// =============================================================================
// BASIC SPAWN CONTEXT TESTS
// =============================================================================

#[tokio::test]
async fn test_codex_subagent_spawn_context_basic() -> Result<()> {
    let mut world = TestWorld::new().with_project("test-project");

    world.enable_provider(TestProvider::Codex)?;
    world.set_cwd("test-project");

    // Add parent session with spawn events
    world.add_session_from_sample(
        TestProvider::Codex,
        "codex_parent_with_spawns.jsonl",
        "rollout-parent.jsonl",
    )?;

    // Add subagent session that matches first spawn (within 100ms window)
    world.add_session_from_sample(
        TestProvider::Codex,
        "codex_subagent_matched.jsonl",
        "rollout-subagent1.jsonl",
    )?;

    let client = initialize_workspace(&world).await?;

    // Find the subagent session (must include children since subagents are filtered by default)
    let sessions = client
        .sessions()
        .list(SessionFilter::all().include_children())?;
    let subagent = sessions
        .iter()
        .find(|s| s.id.contains("subagent-matched-001"))
        .expect("Subagent session should exist");

    // Verify spawn context is set
    assert!(
        subagent.parent_session_id.is_some(),
        "Subagent should have parent_session_id"
    );
    assert!(
        subagent.spawned_by.is_some(),
        "Subagent should have spawn context"
    );

    let spawn_ctx = subagent.spawned_by.as_ref().unwrap();
    // First spawn is at turn 0, step 0 (entered_review_mode is at step 0 after user_message)
    assert_eq!(spawn_ctx.turn_index, 0, "Should be spawned at turn 0");

    Ok(())
}

// =============================================================================
// MULTIPLE SPAWN CORRELATION TESTS
// =============================================================================

#[tokio::test]
async fn test_codex_multiple_subagents_correct_correlation() -> Result<()> {
    let mut world = TestWorld::new().with_project("test-project");

    world.enable_provider(TestProvider::Codex)?;
    world.set_cwd("test-project");

    // Add parent with two spawn events
    world.add_session_from_sample(
        TestProvider::Codex,
        "codex_parent_with_spawns.jsonl",
        "rollout-parent.jsonl",
    )?;

    // Add first subagent (matches first spawn at turn 0)
    world.add_session_from_sample(
        TestProvider::Codex,
        "codex_subagent_matched.jsonl",
        "rollout-sub1.jsonl",
    )?;

    // Add second subagent (matches second spawn at turn 1)
    world.add_session_from_sample(
        TestProvider::Codex,
        "codex_subagent_matched_2.jsonl",
        "rollout-sub2.jsonl",
    )?;

    let client = initialize_workspace(&world).await?;
    // Must include children since subagents are filtered by default
    let sessions = client
        .sessions()
        .list(SessionFilter::all().include_children())?;

    // Find subagents by ID pattern
    let subagent1 = sessions
        .iter()
        .find(|s| s.id.contains("subagent-matched-001"))
        .expect("First subagent should exist");

    let subagent2 = sessions
        .iter()
        .find(|s| s.id.contains("subagent-matched-002"))
        .expect("Second subagent should exist");

    // Both should have parent links
    assert!(subagent1.parent_session_id.is_some());
    assert!(subagent2.parent_session_id.is_some());

    // Both should have spawn context
    let spawn1 = subagent1
        .spawned_by
        .as_ref()
        .expect("Should have spawn_ctx");
    let spawn2 = subagent2
        .spawned_by
        .as_ref()
        .expect("Should have spawn_ctx");

    // First subagent should be linked to turn 0
    assert_eq!(spawn1.turn_index, 0, "First subagent at turn 0");

    // Second subagent should be linked to turn 1
    assert_eq!(spawn2.turn_index, 1, "Second subagent at turn 1");

    Ok(())
}

// =============================================================================
// EDGE CASE: UNMATCHED SUBAGENT
// =============================================================================

#[tokio::test]
async fn test_codex_unmatched_subagent_has_no_spawn_context() -> Result<()> {
    let mut world = TestWorld::new().with_project("test-project");

    world.enable_provider(TestProvider::Codex)?;
    world.set_cwd("test-project");

    // Add parent session
    world.add_session_from_sample(
        TestProvider::Codex,
        "codex_parent_with_spawns.jsonl",
        "rollout-parent.jsonl",
    )?;

    // Add orphan subagent (timestamp at T+2000ms - far from any spawn event)
    world.add_session_from_sample(
        TestProvider::Codex,
        "codex_subagent_unmatched.jsonl",
        "rollout-orphan.jsonl",
    )?;

    let client = initialize_workspace(&world).await?;
    let sessions = client.sessions().list(SessionFilter::all())?;

    let orphan = sessions
        .iter()
        .find(|s| s.id.contains("subagent-orphan"))
        .expect("Orphan subagent should exist");

    // Orphan should NOT have parent link (>100ms from any spawn)
    assert!(
        orphan.parent_session_id.is_none(),
        "Unmatched subagent should NOT have parent_session_id"
    );
    assert!(
        orphan.spawned_by.is_none(),
        "Unmatched subagent should NOT have spawn context"
    );

    Ok(())
}

// =============================================================================
// EDGE CASE: PARENT-ONLY SESSION
// =============================================================================

#[tokio::test]
async fn test_codex_parent_only_session_loads_normally() -> Result<()> {
    let mut world = TestWorld::new().with_project("test-project");

    world.enable_provider(TestProvider::Codex)?;
    world.set_cwd("test-project");

    // Add only the parent session (no subagent files)
    world.add_session_from_sample(
        TestProvider::Codex,
        "codex_parent_with_spawns.jsonl",
        "rollout-parent.jsonl",
    )?;

    let client = initialize_workspace(&world).await?;
    let sessions = client.sessions().list(SessionFilter::all())?;

    // Should have exactly 1 session (the parent)
    assert_eq!(sessions.len(), 1, "Should have exactly 1 session");

    let parent = &sessions[0];
    assert!(
        parent.id.contains("parent-spawn-001"),
        "Should be the parent session"
    );

    // Parent should NOT have parent_session_id (it's not a subagent)
    assert!(
        parent.parent_session_id.is_none(),
        "Parent should not have parent_session_id"
    );

    Ok(())
}

// =============================================================================
// SDK METADATA API TEST
// =============================================================================

#[tokio::test]
async fn test_codex_subagent_metadata_api() -> Result<()> {
    let mut world = TestWorld::new().with_project("test-project");

    world.enable_provider(TestProvider::Codex)?;
    world.set_cwd("test-project");

    // Add parent and matched subagent
    world.add_session_from_sample(
        TestProvider::Codex,
        "codex_parent_with_spawns.jsonl",
        "rollout-parent.jsonl",
    )?;
    world.add_session_from_sample(
        TestProvider::Codex,
        "codex_subagent_matched.jsonl",
        "rollout-subagent.jsonl",
    )?;

    let client = initialize_workspace(&world).await?;

    // Find the subagent via list first to get the session ID
    // Must include children since subagents are filtered by default
    let sessions = client
        .sessions()
        .list(SessionFilter::all().include_children())?;
    let subagent_summary = sessions
        .iter()
        .find(|s| s.id.contains("subagent-matched-001"))
        .expect("Subagent should exist in list");

    // Use get() to get the session handle and then call metadata()
    let session_handle = client.sessions().get(&subagent_summary.id)?;
    let metadata = session_handle
        .metadata()?
        .expect("Subagent should have metadata");

    // Verify metadata contains spawn context
    assert!(
        metadata.parent_session_id.is_some(),
        "Metadata should contain parent_session_id"
    );
    assert!(
        metadata.spawned_by.is_some(),
        "Metadata should contain spawned_by context"
    );

    let spawn_ctx = metadata.spawned_by.unwrap();
    assert_eq!(spawn_ctx.turn_index, 0, "Spawn context turn should be 0");

    Ok(())
}
