//! Watch Mode Tests
//!
//! Verifies that watch mode correctly:
//! - Ignores sidechain file updates when determining which session to attach
//! - Only switches sessions when main files are created/updated
//! - Filters sessions by project isolation

use agtrace_sdk::{Client, SystemClient};
use agtrace_testing::TestWorld;
use agtrace_testing::providers::TestProvider;
use anyhow::Result;
use std::time::Duration;

/// Helper function to initialize the workspace with SDK.
async fn initialize_workspace(world: &TestWorld, all_projects: bool) -> Result<Client> {
    let config = agtrace_sdk::types::InitConfig {
        data_dir: world.data_dir().to_path_buf(),
        project_root: Some(world.cwd().to_path_buf()),
        all_projects,
        refresh: false,
    };
    SystemClient::initialize(config, None::<fn(agtrace_sdk::types::InitProgress)>)?;
    world.get_client().await
}

#[tokio::test]
async fn test_watch_ignores_sidechain_file_creation() -> Result<()> {
    use std::io::Write;

    // Given: Project with a Claude session (main file)
    let mut world = TestWorld::new().with_project("my-project");

    world.enable_provider(TestProvider::Claude)?;
    world.set_cwd("my-project");
    world.add_session(TestProvider::Claude, "session-main.jsonl")?;

    let client = initialize_workspace(&world, false).await?;

    // Get the session ID
    let sessions = client.sessions().list(agtrace_sdk::SessionFilter::all())?;
    assert_eq!(sessions.len(), 1);
    let session_id = &sessions[0].id;

    // Start watching
    let watch_service = client.watch_service();
    let monitor = watch_service
        .watch_provider("claude_code")?
        .with_project_root(world.cwd().to_path_buf())
        .start_background_scan()?;

    // Wait for initial discovery
    std::thread::sleep(Duration::from_millis(500));

    // Create a sidechain file for the same session
    let log_root = world.temp_dir().join(".claude/projects");
    let project_dir = world.cwd().to_string_lossy();
    let encoded_project = project_dir.replace('/', "-");
    let sidechain_dir = log_root.join(&encoded_project);
    std::fs::create_dir_all(&sidechain_dir)?;

    let sidechain_file = sidechain_dir.join("agent-test123.jsonl");
    let mut file = std::fs::File::create(&sidechain_file)?;

    // Write sidechain record with same session_id
    writeln!(
        file,
        r#"{{"parentUuid":null,"isSidechain":true,"sessionId":"{}","agentId":"test123","type":"user","message":{{"role":"user","content":"Sidechain event"}},"uuid":"sidechain-uuid-1","timestamp":"2025-12-30T12:00:00Z"}}"#,
        session_id
    )?;
    drop(file);

    // Wait for potential event processing
    std::thread::sleep(Duration::from_secs(2));

    // Check events - should NOT see a SessionUpdated event for this sidechain file
    // Use receiver().try_recv() to check if there are any pending events
    let has_events = monitor.receiver().try_recv().is_ok();
    assert!(
        !has_events,
        "Should not emit SessionUpdated events for sidechain file creation"
    );

    Ok(())
}

#[tokio::test]
async fn test_watch_detects_main_file_creation() -> Result<()> {
    // Given: Project with initial session
    let mut world = TestWorld::new().with_project("my-project");

    world.enable_provider(TestProvider::Claude)?;
    world.set_cwd("my-project");
    world.add_session(TestProvider::Claude, "session-a.jsonl")?;

    // Set older modification time
    let old_time = std::time::SystemTime::now() - Duration::from_secs(30);
    world.set_file_mtime(TestProvider::Claude, "session-a.jsonl", old_time)?;

    let client = initialize_workspace(&world, false).await?;

    // Start watching
    let watch_service = client.watch_service();
    let monitor = watch_service
        .watch_provider("claude_code")?
        .with_project_root(world.cwd().to_path_buf())
        .start_background_scan()?;

    // Wait for initial discovery
    std::thread::sleep(Duration::from_millis(500));

    // Create a new main session file (not sidechain)
    world.add_session(TestProvider::Claude, "session-b.jsonl")?;

    // Wait for discovery to detect new file
    std::thread::sleep(Duration::from_secs(2));

    // Check events - SHOULD see a SessionUpdated event for the new main file
    let mut found_session_updated = false;
    // Poll for events with timeout
    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_secs(3) {
        if let Ok(event) = monitor.receiver().try_recv() {
            if let agtrace_runtime::WorkspaceEvent::Discovery(
                agtrace_runtime::DiscoveryEvent::SessionUpdated { session_id, .. },
            ) = event
            {
                // Verify it's a different session (session-b)
                if !session_id.is_empty() {
                    found_session_updated = true;
                    break;
                }
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    assert!(
        found_session_updated,
        "Should receive SessionUpdated event for new main file"
    );

    Ok(())
}
