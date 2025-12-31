//! Filtering Tests
//!
//! Verifies that session list filtering works correctly
//! with provider, project, and limit filters.

use agtrace_sdk::{Client, SessionFilter, SystemClient};
use agtrace_testing::TestWorld;
use agtrace_testing::providers::TestProvider;
use anyhow::Result;

/// Helper function to initialize the workspace with SDK.
/// Always initializes with all_projects=true for maximum flexibility in filtering tests.
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
// PROVIDER FILTERING TESTS
// =============================================================================

#[tokio::test]
async fn test_list_filter_by_source_provider() -> Result<()> {
    let mut world = TestWorld::new().with_project("my-project");

    world.enable_provider(TestProvider::Claude)?;
    world.enable_provider(TestProvider::Gemini)?;

    world.set_cwd("my-project");
    world.add_session(TestProvider::Claude, "claude-session.jsonl")?;
    world.add_session(TestProvider::Gemini, "session-gemini.json")?;

    let client = initialize_workspace(&world).await?;

    // Get project hash for filtering to current project
    let project_path = world.temp_dir().join("my-project");
    let project_hash = agtrace_sdk::utils::project_hash_from_root(&project_path.to_string_lossy());

    let filter = SessionFilter::default()
        .project(project_hash.clone())
        .provider("claude_code".to_string());
    let sessions = client.sessions().list(filter)?;

    assert_eq!(sessions.len(), 1, "Should show only Claude sessions");
    assert_eq!(sessions[0].provider, "claude_code");

    let filter = SessionFilter::default()
        .project(project_hash)
        .provider("gemini".to_string());
    let sessions = client.sessions().list(filter)?;

    assert_eq!(sessions.len(), 1, "Should show only Gemini sessions");
    assert_eq!(sessions[0].provider, "gemini");

    Ok(())
}

#[tokio::test]
async fn test_session_list_without_provider_shows_all_providers() -> Result<()> {
    let mut world = TestWorld::new().with_project("my-project");

    world.enable_provider(TestProvider::Claude)?;
    world.enable_provider(TestProvider::Gemini)?;

    world.set_cwd("my-project");
    world.add_session(TestProvider::Claude, "claude-session.jsonl")?;
    world.add_session(TestProvider::Gemini, "session-gemini.json")?;

    let client = initialize_workspace(&world).await?;

    // Filter to current project but all providers
    let project_path = world.temp_dir().join("my-project");
    let project_hash = agtrace_sdk::utils::project_hash_from_root(&project_path.to_string_lossy());

    let filter = SessionFilter::default().project(project_hash);
    let sessions = client.sessions().list(filter)?;

    assert_eq!(sessions.len(), 2, "Should show sessions from all providers");

    Ok(())
}

#[tokio::test]
async fn test_provider_filter_with_disabled_provider_shows_no_sessions() -> Result<()> {
    let mut world = TestWorld::new().with_project("my-project");

    world.enable_provider(TestProvider::Claude)?;

    world.set_cwd("my-project");
    world.add_session(TestProvider::Claude, "claude-session.jsonl")?;

    let client = initialize_workspace(&world).await?;

    let project_path = world.temp_dir().join("my-project");
    let project_hash = agtrace_sdk::utils::project_hash_from_root(&project_path.to_string_lossy());

    let filter = SessionFilter::default()
        .project(project_hash)
        .provider("gemini".to_string());
    let sessions = client.sessions().list(filter)?;

    assert_eq!(
        sessions.len(),
        0,
        "Should show no sessions for disabled provider"
    );

    Ok(())
}

// =============================================================================
// PROJECT FILTERING TESTS
// =============================================================================

#[tokio::test]
async fn test_list_all_projects_shows_sessions_from_all_projects() -> Result<()> {
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

    let client = initialize_workspace(&world).await?;

    // Use all_projects filter to show all
    let sessions = client
        .sessions()
        .list(SessionFilter::default().all_projects())?;

    assert_eq!(sessions.len(), 3, "Should show all projects' sessions");

    Ok(())
}

#[tokio::test]
async fn test_list_without_all_projects_shows_only_current_project() -> Result<()> {
    let mut world = TestWorld::new()
        .with_project("project-a")
        .with_project("project-b");

    world.enable_provider(TestProvider::Claude)?;

    world.set_cwd("project-a");
    world.add_session(TestProvider::Claude, "session-a1.jsonl")?;
    world.add_session(TestProvider::Claude, "session-a2.jsonl")?;

    world.set_cwd("project-b");
    world.add_session(TestProvider::Claude, "session-b1.jsonl")?;

    let client = initialize_workspace(&world).await?;

    // Filter by project-a's hash
    let project_a_path = world.temp_dir().join("project-a");
    let expected_hash =
        agtrace_sdk::utils::project_hash_from_root(&project_a_path.to_string_lossy());

    let filter = SessionFilter::default().project(expected_hash.clone());
    let sessions = client.sessions().list(filter)?;

    assert_eq!(sessions.len(), 2, "Should show only project-a's sessions");

    for session in &sessions {
        assert_eq!(
            session.project_hash, expected_hash,
            "All sessions should belong to project-a"
        );
    }

    Ok(())
}

// =============================================================================
// LIMIT FILTERING TESTS
// =============================================================================

#[tokio::test]
async fn test_list_limit_parameter() -> Result<()> {
    let mut world = TestWorld::new().with_project("my-project");

    world.enable_provider(TestProvider::Claude)?;
    world.set_cwd("my-project");

    for i in 1..=5 {
        world.add_session(TestProvider::Claude, &format!("session-{}.jsonl", i))?;
    }

    let client = initialize_workspace(&world).await?;

    let project_path = world.temp_dir().join("my-project");
    let project_hash = agtrace_sdk::utils::project_hash_from_root(&project_path.to_string_lossy());

    let filter = SessionFilter::default().project(project_hash).limit(3);
    let sessions = client.sessions().list(filter)?;

    assert_eq!(sessions.len(), 3, "Should show only 3 sessions");

    Ok(())
}

// =============================================================================
// COMBINED FILTERS TESTS
// =============================================================================

#[tokio::test]
async fn test_list_combined_filters() -> Result<()> {
    let mut world = TestWorld::new()
        .with_project("project-a")
        .with_project("project-b");

    world.enable_provider(TestProvider::Claude)?;
    world.enable_provider(TestProvider::Gemini)?;

    world.set_cwd("project-a");
    world.add_session(TestProvider::Claude, "claude-a1.jsonl")?;
    world.add_session(TestProvider::Claude, "claude-a2.jsonl")?;

    world.set_cwd("project-b");
    world.add_session(TestProvider::Claude, "claude-b1.jsonl")?;
    world.add_session(TestProvider::Gemini, "session-b1.json")?;

    let client = initialize_workspace(&world).await?;

    // Filter all projects, only Claude provider
    let filter = SessionFilter::default()
        .all_projects()
        .provider("claude_code".to_string());
    let sessions = client.sessions().list(filter)?;

    assert_eq!(
        sessions.len(),
        3,
        "Should show only Claude sessions from all projects"
    );

    for session in &sessions {
        assert_eq!(
            session.provider, "claude_code",
            "All sessions should be from claude_code"
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_combined_filters_provider_and_project() -> Result<()> {
    let mut world = TestWorld::new()
        .with_project("project-a")
        .with_project("project-b");

    world.enable_provider(TestProvider::Claude)?;
    world.enable_provider(TestProvider::Gemini)?;

    world.set_cwd("project-a");
    world.add_session(TestProvider::Claude, "claude-a.jsonl")?;
    world.add_session(TestProvider::Gemini, "gemini-a.json")?;

    world.set_cwd("project-b");
    world.add_session(TestProvider::Claude, "claude-b.jsonl")?;

    let client = initialize_workspace(&world).await?;

    // Filter by project-a's hash AND claude_code provider
    let project_a_path = world.temp_dir().join("project-a");
    let project_a_hash =
        agtrace_sdk::utils::project_hash_from_root(&project_a_path.to_string_lossy());

    let filter = SessionFilter::default()
        .project(project_a_hash)
        .provider("claude_code".to_string());
    let sessions = client.sessions().list(filter)?;

    assert_eq!(
        sessions.len(),
        1,
        "Should show only Claude sessions from current project"
    );
    assert_eq!(sessions[0].provider, "claude_code");

    Ok(())
}

// =============================================================================
// EDGE CASES
// =============================================================================

#[tokio::test]
async fn test_list_no_sessions_returns_empty_array() -> Result<()> {
    let mut world = TestWorld::new().with_project("empty-project");

    world.enable_provider(TestProvider::Claude)?;
    world.set_cwd("empty-project");

    let client = initialize_workspace(&world).await?;

    let project_path = world.temp_dir().join("empty-project");
    let project_hash = agtrace_sdk::utils::project_hash_from_root(&project_path.to_string_lossy());

    let filter = SessionFilter::default().project(project_hash);
    let sessions = client.sessions().list(filter)?;

    assert_eq!(
        sessions.len(),
        0,
        "Should return empty array for no sessions"
    );

    Ok(())
}
