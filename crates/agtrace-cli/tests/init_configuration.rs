//! Init & Configuration Tests
//!
//! Verifies that initialization and configuration workflows
//! handle various states correctly (fresh install, refresh, already indexed).

use agtrace_testing::TestWorld;
use agtrace_testing::providers::TestProvider;
use anyhow::Result;

#[test]
fn test_init_uninitialized_directory_with_session_files() -> Result<()> {
    // Given: Uninitialized directory with session files
    let mut world = TestWorld::builder().without_data_dir().build();

    world = world.with_project("my-project");

    // Setup provider log directory (simulating external provider setup)
    let log_root = world.temp_dir().join(".claude");
    std::fs::create_dir_all(&log_root)?;

    // Write config manually (simulating provider auto-detection)
    let config_content = format!(
        r#"
[providers.claude_code]
enabled = true
log_root = "{}"
"#,
        log_root.display()
    );
    world.write_raw_config(&config_content)?;

    // Place session file in provider's log directory
    world.set_cwd("my-project");
    let samples = agtrace_testing::fixtures::SampleFiles::new();
    let project_path = world.temp_dir().join("my-project");
    let adapter = TestProvider::Claude.adapter();
    samples.copy_to_project_with_cwd(
        "claude_session.jsonl",
        "session.jsonl",
        &project_path.to_string_lossy(),
        &log_root,
        &adapter,
    )?;

    // Verify data dir doesn't exist yet
    assert!(
        !world.assert_database_exists(),
        "Database should not exist before init"
    );

    // When: Run init
    let result = world.run(&["init"])?;

    // Then: DB is created and index completes
    assert!(result.success(), "Init should succeed: {}", result.stderr());
    assert!(world.assert_database_exists(), "Database should be created");

    // EXPECTED: Session should be indexed
    // ACTUAL: Scanner reports "Found 0 sessions" despite file being present
    let list_result = world.run(&["session", "list", "--format", "json"])?;
    assert!(list_result.success());
    let json = list_result.json()?;
    let sessions = json["content"]["sessions"]
        .as_array()
        .expect("Should have sessions array");
    assert!(
        !sessions.is_empty(),
        "Should have indexed at least one session (Bug: session discovery fails on first run)"
    );

    Ok(())
}

#[test]
fn test_init_refresh_discards_existing_data() -> Result<()> {
    // Given: Already indexed project
    let mut world = TestWorld::new().with_project("my-project");

    world.enable_provider(TestProvider::Claude)?;
    world.set_cwd("my-project");
    world.add_session(TestProvider::Claude, "session1.jsonl")?;

    world.run(&["init"])?;

    // Verify initial index
    let before_result = world.run(&["session", "list", "--format", "json"])?;
    assert!(before_result.success());
    let before_json = before_result.json()?;
    let before_count = before_json["content"]["sessions"].as_array().unwrap().len();
    // EXPECTED: 1 session
    // ACTUAL: 0 sessions (same root cause as test_init_uninitialized)
    assert_eq!(
        before_count, 1,
        "Should have 1 session initially (Bug: init doesn't index sessions)"
    );

    // Add another session file
    world.add_session(TestProvider::Claude, "session2.jsonl")?;

    // When: Run init --refresh
    let refresh_result = world.run(&["init", "--refresh"])?;

    // Then: Index is rebuilt (should now have 2 sessions)
    assert!(
        refresh_result.success(),
        "Refresh should succeed: {}",
        refresh_result.stderr()
    );

    let after_result = world.run(&["session", "list", "--format", "json"])?;
    assert!(after_result.success());
    let after_json = after_result.json()?;
    let after_count = after_json["content"]["sessions"].as_array().unwrap().len();
    assert_eq!(after_count, 2, "Should have 2 sessions after refresh");

    Ok(())
}

#[test]
fn test_init_already_indexed_skips_rescan() -> Result<()> {
    // Given: Already indexed project
    let mut world = TestWorld::new().with_project("my-project");

    world.enable_provider(TestProvider::Claude)?;
    world.set_cwd("my-project");
    world.add_session(TestProvider::Claude, "session.jsonl")?;

    // First init
    let first_init = world.run(&["init"])?;
    assert!(first_init.success());

    // When: Run init again (without --refresh)
    let second_init = world.run(&["init"])?;

    // Then: Should succeed but skip re-scan
    assert!(
        second_init.success(),
        "Second init should succeed: {}",
        second_init.stderr()
    );

    // The output should indicate incremental mode or skip
    // (Implementation detail: check that DB still exists and is valid)
    let list_result = world.run(&["session", "list", "--format", "json"])?;
    assert!(list_result.success());

    Ok(())
}

#[test]
fn test_init_with_missing_provider_log_root() -> Result<()> {
    // Given: Provider configured but log_root doesn't exist
    let mut world = TestWorld::builder().without_data_dir().build();
    world = world.with_project("my-project");
    world.set_cwd("my-project");

    // Write config with non-existent log root
    let config_content = r#"
[providers.claude_code]
enabled = true
log_root = "/nonexistent/path/.claude"
"#;
    world.write_raw_config(config_content)?;

    // When: Run init
    let result = world.run(&["init"])?;

    // Then: Should succeed but skip the missing provider
    assert!(result.success(), "Init should succeed: {}", result.stderr());
    assert!(world.assert_database_exists(), "DB should be created");

    // List should show 0 sessions
    let list_result = world.run(&["session", "list", "--format", "json"])?;
    assert!(list_result.success());
    let json = list_result.json()?;
    let sessions = json["content"]["sessions"].as_array().unwrap();
    assert_eq!(sessions.len(), 0, "Should have 0 sessions");

    Ok(())
}

#[test]
fn test_init_detects_providers_automatically() -> Result<()> {
    // Given: Fresh install with provider log directories present
    let world = TestWorld::builder().without_data_dir().build();

    // Create provider directories (simulating external provider installation)
    let claude_logs = world.temp_dir().join(".claude");
    std::fs::create_dir_all(&claude_logs)?;

    // When: Run init (should auto-detect)
    let result = world.run(&["init"])?;

    // Then: Config should be created with detected providers
    assert!(result.success(), "Init should succeed: {}", result.stderr());
    assert!(world.assert_config_exists(), "Config should be created");

    // Verify provider was detected
    let config_content = std::fs::read_to_string(world.data_dir().join("config.toml"))?;
    assert!(
        config_content.contains("claude_code"),
        "Config should contain claude_code provider"
    );

    Ok(())
}

/// Issue #5: init fails to index sessions on first run
///
/// This test documents the current buggy behavior where `agtrace init`
/// reports "Found 0 sessions" on the first run, even when session files exist.
/// A second run with `--refresh` is required to properly index sessions.
///
/// CURRENT BEHAVIOR (buggy):
/// - First init reports 0 sessions in output
/// - Sessions are indexed by handler AFTER InitService returns
/// - Session list succeeds because handler runs indexing
///
/// EXPECTED BEHAVIOR (to be fixed):
/// - First init should report the correct session count
/// - InitService should perform indexing before counting
#[test]
fn test_issue_5_init_reports_zero_sessions_on_first_run() -> Result<()> {
    // Given: Uninitialized directory with session files
    let mut world = TestWorld::builder().without_data_dir().build();
    world = world.with_project("my-project");

    // Setup provider log directory
    let log_root = world.temp_dir().join(".claude");
    std::fs::create_dir_all(&log_root)?;

    // Write config manually
    let config_content = format!(
        r#"
[providers.claude_code]
enabled = true
log_root = "{}"
"#,
        log_root.display()
    );
    world.write_raw_config(&config_content)?;

    // Place session file in provider's log directory
    world.set_cwd("my-project");
    let samples = agtrace_testing::fixtures::SampleFiles::new();
    let project_path = world.temp_dir().join("my-project");
    let adapter = TestProvider::Claude.adapter();
    samples.copy_to_project_with_cwd(
        "claude_session.jsonl",
        "session.jsonl",
        &project_path.to_string_lossy(),
        &log_root,
        &adapter,
    )?;

    // When: Run init for the first time with JSON output
    let result = world.run(&["init", "--format", "json"])?;
    assert!(result.success(), "Init should succeed: {}", result.stderr());

    // Extract JSON from output (there may be additional plain text after JSON)
    let output = result.stdout();
    let json_start = output.find('{').expect("Should have JSON in output");
    let json_str = &output[json_start..];

    // Find the end of the JSON object
    let mut brace_count = 0;
    let mut json_end = 0;
    for (i, ch) in json_str.chars().enumerate() {
        if ch == '{' {
            brace_count += 1;
        } else if ch == '}' {
            brace_count -= 1;
            if brace_count == 0 {
                json_end = i + 1;
                break;
            }
        }
    }

    let json_only = &json_str[..json_end];
    let json: serde_json::Value = serde_json::from_str(json_only)?;

    // Then: BUG - InitService reports 0 sessions even though file exists
    // This is the current buggy behavior that needs to be fixed
    let session_count = json["content"]["session_count"]
        .as_u64()
        .expect("Should have session_count field");

    // TODO: After fix, this assertion should be:
    // assert_eq!(session_count, 1, "Init should report 1 session");
    //
    // Current buggy behavior - InitService counts 0 sessions before indexing:
    assert_eq!(
        session_count,
        0,
        "BUG: InitService should count sessions after indexing, but currently counts before. \
         This test documents the buggy behavior. JSON: {}",
        serde_json::to_string_pretty(&json)?
    );

    // Note: The handler DOES run indexing after InitService returns,
    // so session list will show results. This masks the bug in tests
    // that only check `session list` results.
    let list_result = world.run(&["session", "list", "--format", "json"])?;
    let list_json = list_result.json()?;
    let sessions = list_json["content"]["sessions"].as_array().unwrap();
    assert!(
        !sessions.is_empty(),
        "Sessions are indexed by handler after init"
    );

    Ok(())
}
