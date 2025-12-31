//! Project Isolation Tests
//!
//! Verifies that sessions are correctly isolated by project hash,
//! preventing cross-project data leakage.

use agtrace_sdk::{Client, SessionFilter, SystemClient};
use agtrace_testing::TestWorld;
use agtrace_testing::providers::TestProvider;
use anyhow::Result;

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
async fn test_isolation_project_a_and_b_list_shows_only_current_project() -> Result<()> {
    let mut world = TestWorld::new()
        .with_project("project-a")
        .with_project("project-b");

    world.enable_provider(TestProvider::Claude)?;

    world.set_cwd("project-a");
    world.add_session(TestProvider::Claude, "session-a1.jsonl")?;

    world.set_cwd("project-b");
    world.add_session(TestProvider::Claude, "session-b1.jsonl")?;

    // Initialize with all projects, then we'll filter by project hash
    let client = initialize_workspace(&world, true).await?;

    // Filter by project-a's hash
    let project_a_path = world.temp_dir().join("project-a");
    let expected_hash =
        agtrace_sdk::utils::project_hash_from_root(&project_a_path.to_string_lossy());

    let filter = SessionFilter::default().project(expected_hash.clone());
    let sessions = client.sessions().list(filter)?;

    assert_eq!(sessions.len(), 1, "Should show only project A's sessions");
    assert_eq!(
        sessions[0].project_hash, expected_hash,
        "Session should belong to project A"
    );

    Ok(())
}

#[tokio::test]
async fn test_isolation_empty_project_shows_zero_results() -> Result<()> {
    let mut world = TestWorld::new()
        .with_project("project-a")
        .with_project("project-b");

    world.enable_provider(TestProvider::Claude)?;

    world.set_cwd("project-a");
    world.add_session(TestProvider::Claude, "session-a1.jsonl")?;

    let client = initialize_workspace(&world, true).await?;

    // Filter by project-b's hash (empty project)
    let project_b_path = world.temp_dir().join("project-b");
    let project_b_hash =
        agtrace_sdk::utils::project_hash_from_root(&project_b_path.to_string_lossy());

    let filter = SessionFilter::default().project(project_b_hash);
    let sessions = client.sessions().list(filter)?;

    assert_eq!(
        sessions.len(),
        0,
        "Should show no sessions for empty project"
    );

    Ok(())
}

#[tokio::test]
async fn test_isolation_multiple_sessions_in_same_project() -> Result<()> {
    let mut world = TestWorld::new().with_project("my-project");

    world.enable_provider(TestProvider::Claude)?;

    world.set_cwd("my-project");
    world.add_session(TestProvider::Claude, "session1.jsonl")?;
    world.add_session(TestProvider::Claude, "session2.jsonl")?;
    world.add_session(TestProvider::Claude, "session3.jsonl")?;

    // Initialize with the current project directory set
    let client = initialize_workspace(&world, false).await?;

    let project_path = world.temp_dir().join("my-project");
    let expected_hash = agtrace_sdk::utils::project_hash_from_root(&project_path.to_string_lossy());

    let filter = SessionFilter::default().project(expected_hash.clone());
    let sessions = client.sessions().list(filter)?;

    assert_eq!(sessions.len(), 3, "Should show all sessions in the project");

    for session in &sessions {
        assert_eq!(
            session.project_hash, expected_hash,
            "All sessions should belong to this project"
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_isolation_all_projects_flag_shows_all_sessions() -> Result<()> {
    let mut world = TestWorld::new()
        .with_project("project-a")
        .with_project("project-b");

    world.enable_provider(TestProvider::Claude)?;

    world.set_cwd("project-a");
    world.add_session(TestProvider::Claude, "session-a.jsonl")?;

    world.set_cwd("project-b");
    world.add_session(TestProvider::Claude, "session-b.jsonl")?;

    let client = initialize_workspace(&world, true).await?;

    // Use all_projects filter to get sessions from all projects
    let sessions = client
        .sessions()
        .list(SessionFilter::default().all_projects())?;

    assert_eq!(sessions.len(), 2, "Should show both projects' sessions");

    Ok(())
}
