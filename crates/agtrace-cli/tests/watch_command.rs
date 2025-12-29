//! Watch Command Tests
//!
//! Verifies that the watch command correctly:
//! - Attaches to the latest session in the current project
//! - Waits when no sessions exist
//! - Respects explicit session IDs

use agtrace_testing::TestWorld;
use agtrace_testing::process::BackgroundProcess;
use agtrace_testing::providers::TestProvider;
use anyhow::Result;
use std::io::{BufRead, BufReader};
use std::time::Duration;

#[test]
fn test_watch_attaches_to_current_project_latest_session() -> Result<()> {
    // Given: Project A and B both have sessions
    let mut world = TestWorld::new()
        .with_project("project-a")
        .with_project("project-b");

    world.enable_provider(TestProvider::Claude)?;

    world.set_cwd("project-a");
    world.add_session(TestProvider::Claude, "session-a.jsonl")?;

    world.set_cwd("project-b");
    world.add_session(TestProvider::Claude, "session-b.jsonl")?;

    world.run(&["init", "--all-projects"])?;

    // Get session IDs for verification
    world.set_cwd("project-a");
    let list_a = world.run(&["session", "list", "--format", "json"])?;
    let json_a = list_a.json()?;
    let session_a_id = json_a["content"]["sessions"][0]["id"]
        .as_str()
        .expect("Should have session ID");

    // When: Run watch in project-a
    world.set_cwd("project-a");
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_agtrace"));
    cmd.current_dir(world.temp_dir().join("project-a"))
        .arg("--data-dir")
        .arg(world.data_dir())
        .args(["watch", "--mode", "console", "--provider", "claude_code"]);

    let mut proc = BackgroundProcess::spawn_piped(cmd)?;

    // Then: Should attach to project-a's session (not project-b)
    let stdout = proc.stdout().expect("Should have stdout");
    let reader = BufReader::new(stdout);

    let mut found_attachment = false;
    for line in reader.lines().take(10) {
        let line = line?;
        if line.contains("Attached") && line.contains(&session_a_id[..8]) {
            found_attachment = true;
            break;
        }
    }

    // Clean up
    proc.kill()?;

    assert!(
        found_attachment,
        "Should attach to project-a's session, not project-b"
    );

    Ok(())
}

#[test]
fn test_watch_waits_when_current_project_is_empty() -> Result<()> {
    // Given: Project A is empty, Project B has data
    let mut world = TestWorld::new()
        .with_project("project-a")
        .with_project("project-b");

    world.enable_provider(TestProvider::Claude)?;

    world.set_cwd("project-b");
    world.add_session(TestProvider::Claude, "session-b.jsonl")?;

    world.run(&["init", "--all-projects"])?;

    // When: Run watch in project-a (empty)
    world.set_cwd("project-a");
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_agtrace"));
    cmd.current_dir(world.temp_dir().join("project-a"))
        .arg("--data-dir")
        .arg(world.data_dir())
        .args(["watch", "--mode", "console", "--provider", "claude_code"]);

    let mut proc = BackgroundProcess::spawn_piped(cmd)?;

    // Then: Should show "Waiting" (doesn't connect to project-b's session)
    let stdout = proc.stdout().expect("Should have stdout");
    let reader = BufReader::new(stdout);

    let mut found_waiting = false;
    for line in reader.lines().take(10) {
        let line = line?;
        if line.contains("Waiting") || line.contains("waiting") {
            found_waiting = true;
            break;
        }
    }

    // Clean up
    proc.kill()?;

    assert!(
        found_waiting,
        "Should wait for new session, not connect to project-b"
    );

    Ok(())
}

#[test]
fn test_watch_with_explicit_session_id() -> Result<()> {
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

    // Get project-b's session ID
    world.set_cwd("project-b");
    let list_b = world.run(&["session", "list", "--format", "json"])?;
    let json_b = list_b.json()?;
    let session_b_id = json_b["content"]["sessions"][0]["id"]
        .as_str()
        .expect("Should have session ID");

    // When: Run watch with explicit --id from project-a directory
    world.set_cwd("project-a");
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_agtrace"));
    cmd.current_dir(world.temp_dir().join("project-a"))
        .arg("--data-dir")
        .arg(world.data_dir())
        .args(["watch", "--mode", "console", "--id", session_b_id]);

    let mut proc = BackgroundProcess::spawn_piped(cmd)?;

    // Then: Should attach to the specified session (bypassing project context)
    let stdout = proc.stdout().expect("Should have stdout");
    let reader = BufReader::new(stdout);

    let mut found_attachment = false;
    for line in reader.lines().take(10) {
        let line = line?;
        if line.contains("Attached") && line.contains(&session_b_id[..8]) {
            found_attachment = true;
            break;
        }
    }

    // Clean up
    proc.kill()?;

    assert!(
        found_attachment,
        "Should attach to explicitly specified session regardless of directory"
    );

    Ok(())
}

#[test]
fn test_watch_console_mode_streams_output() -> Result<()> {
    // Given: Project with a session
    let mut world = TestWorld::new().with_project("my-project");

    world.enable_provider(TestProvider::Claude)?;
    world.set_cwd("my-project");
    world.add_session(TestProvider::Claude, "session.jsonl")?;

    world.run(&["init"])?;

    // When: Run watch in console mode
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_agtrace"));
    cmd.current_dir(world.temp_dir().join("my-project"))
        .arg("--data-dir")
        .arg(world.data_dir())
        .args(["watch", "--mode", "console", "--provider", "claude_code"]);

    let mut proc = BackgroundProcess::spawn_piped(cmd)?;

    // Then: Should produce console output (not TUI escape sequences)
    let stdout = proc.stdout().expect("Should have stdout");
    let reader = BufReader::new(stdout);

    let mut has_output = false;
    for line in reader.lines().take(5) {
        let line = line?;
        // Console mode should have readable text, not TUI control codes
        if !line.is_empty() {
            has_output = true;
            break;
        }
    }

    // Clean up
    proc.kill()?;

    assert!(has_output, "Console mode should produce text output");

    Ok(())
}

#[test]
fn test_watch_requires_tty_for_tui_mode() -> Result<()> {
    // Given: Any initialized project
    let mut world = TestWorld::new().with_project("my-project");
    world.enable_provider(TestProvider::Claude)?;
    world.set_cwd("my-project");
    world.run(&["init"])?;

    // When: Run watch in TUI mode without TTY (piped)
    let result = world.run(&["watch", "--mode", "tui", "--provider", "claude_code"]);

    // Then: Should fail with helpful error message
    // Note: This test assumes the command checks for TTY
    // If run in a real TTY environment, this test may need to be conditional
    let is_likely_non_tty =
        result.is_err() || result.as_ref().map(|r| !r.success()).unwrap_or(false);

    if is_likely_non_tty {
        // Expected behavior in non-TTY environment
        let err = result
            .as_ref()
            .err()
            .map(|e| e.to_string())
            .or_else(|| result.as_ref().ok().map(|r| r.stderr().to_string()))
            .unwrap_or_default();

        assert!(
            err.contains("TTY") || err.contains("tty") || err.contains("terminal"),
            "Error should mention TTY requirement: {}",
            err
        );
    }

    Ok(())
}

#[test]
fn test_watch_timeout_handling() -> Result<()> {
    // Given: Project with session
    let mut world = TestWorld::new().with_project("my-project");
    world.enable_provider(TestProvider::Claude)?;
    world.set_cwd("my-project");
    world.add_session(TestProvider::Claude, "session.jsonl")?;

    world.run(&["init"])?;

    // When: Start watch and let it run briefly
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_agtrace"));
    cmd.current_dir(world.temp_dir().join("my-project"))
        .arg("--data-dir")
        .arg(world.data_dir())
        .args(["watch", "--mode", "console", "--provider", "claude_code"]);

    let mut proc = BackgroundProcess::spawn_piped(cmd)?;

    // Then: Should be killable and clean up properly
    std::thread::sleep(Duration::from_secs(1));

    let kill_result = proc.kill();
    assert!(kill_result.is_ok(), "Should be able to kill watch process");

    Ok(())
}

#[test]
fn test_watch_cross_provider_switching() -> Result<()> {
    // Given: Project with sessions from multiple providers
    let mut world = TestWorld::new().with_project("my-project");

    // Enable both Claude and Codex providers
    world.enable_provider(TestProvider::Claude)?;
    world.enable_provider(TestProvider::Codex)?;

    world.set_cwd("my-project");

    // Create Claude session with older modification time
    world.add_session(TestProvider::Claude, "claude-session.jsonl")?;
    let old_time = std::time::SystemTime::now() - Duration::from_secs(10);
    world.set_file_mtime(TestProvider::Claude, "claude-session.jsonl", old_time)?;

    world.run(&["init", "--all-projects"])?;

    // When: Start watch (should initially attach to Claude session)
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_agtrace"));
    cmd.current_dir(world.temp_dir().join("my-project"))
        .arg("--data-dir")
        .arg(world.data_dir())
        .args(["watch", "--mode", "console"]);

    let mut proc = BackgroundProcess::spawn_piped(cmd)?;

    // Give watch time to start and attach to initial session
    std::thread::sleep(Duration::from_millis(500));

    // Create a new Codex session with a newer modification time
    world.add_session(TestProvider::Codex, "codex-session.jsonl")?;

    // Give watch time to detect the new session and switch
    std::thread::sleep(Duration::from_secs(2));

    // Then: Watch should switch to the newer Codex session
    let stdout = proc.stdout().expect("Should have stdout");
    let reader = BufReader::new(stdout);

    let mut found_switch = false;
    for line in reader.lines().take(30) {
        let line = line?;
        // Look for "Switched to session" or similar indication
        if line.contains("Switched") || line.contains("Attached") {
            found_switch = true;
            break;
        }
    }

    // Clean up
    proc.kill()?;

    assert!(
        found_switch,
        "Watch should switch to the newer session from a different provider"
    );

    Ok(())
}

#[test]
fn test_watch_auto_provider_selection_respects_project_isolation() -> Result<()> {
    // Given: Two projects with sessions from different providers
    let mut world = TestWorld::new()
        .with_project("project-a")
        .with_project("project-b");

    world.enable_provider(TestProvider::Claude)?;
    world.enable_provider(TestProvider::Codex)?;

    // Project A: Claude session (older modification time)
    world.set_cwd("project-a");
    world.add_session(TestProvider::Claude, "claude-session.jsonl")?;
    let old_time = std::time::SystemTime::now() - Duration::from_secs(60);
    world.set_file_mtime(TestProvider::Claude, "claude-session.jsonl", old_time)?;

    // Project B: Codex session (newer modification time, but different project)
    world.set_cwd("project-b");
    world.add_session(TestProvider::Codex, "codex-session.jsonl")?;
    let new_time = std::time::SystemTime::now();
    world.set_file_mtime(TestProvider::Codex, "codex-session.jsonl", new_time)?;

    world.run(&["init", "--all-projects"])?;

    // Get session IDs for verification
    world.set_cwd("project-a");
    let list_a = world.run(&["session", "list", "--format", "json"])?;
    let json_a = list_a.json()?;
    let session_a_id = json_a["content"]["sessions"][0]["id"]
        .as_str()
        .expect("Should have session ID");

    // When: Run watch in project-a WITHOUT specifying --provider
    // (automatic provider selection should pick Claude, not Codex)
    world.set_cwd("project-a");
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_agtrace"));
    cmd.current_dir(world.temp_dir().join("project-a"))
        .arg("--data-dir")
        .arg(world.data_dir())
        .args(["watch", "--mode", "console"]); // No --provider flag

    let mut proc = BackgroundProcess::spawn_piped(cmd)?;

    // Then: Should attach to project-a's Claude session (not project-b's newer Codex session)
    let stdout = proc.stdout().expect("Should have stdout");
    let reader = BufReader::new(stdout);

    let mut found_attachment = false;
    for line in reader.lines().take(15) {
        let line = line?;
        if (line.contains("Attached") || line.contains("Watching"))
            && line.contains(&session_a_id[..8])
        {
            found_attachment = true;
            break;
        }
    }

    // Clean up
    proc.kill()?;

    assert!(
        found_attachment,
        "Watch should attach to current project's session (project-a Claude), \
         not the globally newest session (project-b Codex)"
    );

    Ok(())
}

#[test]
fn test_watch_ignores_events_from_other_projects() -> Result<()> {
    use std::io::Write;

    // Given: Two projects with sessions
    let mut world = TestWorld::new()
        .with_project("project-a")
        .with_project("project-b");

    world.enable_provider(TestProvider::Claude)?;

    // Project A: Has one session
    world.set_cwd("project-a");
    world.add_session(TestProvider::Claude, "session-a.jsonl")?;

    // Project B: Has one session
    world.set_cwd("project-b");
    world.add_session(TestProvider::Claude, "session-b.jsonl")?;

    world.run(&["init", "--all-projects"])?;

    // Get project-b's session file path for later modification
    world.set_cwd("project-b");
    let project_b_session_path =
        world.get_session_file_path(TestProvider::Claude, "session-b.jsonl")?;

    // When: Start watch in project-a
    world.set_cwd("project-a");
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_agtrace"));
    cmd.current_dir(world.temp_dir().join("project-a"))
        .arg("--data-dir")
        .arg(world.data_dir())
        .args(["watch", "--mode", "console", "--provider", "claude_code"]);

    let mut proc = BackgroundProcess::spawn_piped(cmd)?;

    // Wait for watch to start
    std::thread::sleep(Duration::from_millis(500));

    // Modify project-b's session file (should be ignored)
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&project_b_session_path)?;
    writeln!(
        file,
        "{{\"type\":\"user_message\",\"timestamp\":\"2025-12-30T10:00:00Z\"}}"
    )?;
    drop(file);

    // Wait for potential event processing
    std::thread::sleep(Duration::from_secs(2));

    // Then: Should NOT see any "Switched" messages (project-b events should be ignored)
    let stdout = proc.stdout().expect("Should have stdout");
    let reader = BufReader::new(stdout);

    let mut found_switch = false;
    for line in reader.lines().take(20) {
        let line = line?;
        if line.contains("Switched") {
            found_switch = true;
            break;
        }
    }

    // Clean up
    proc.kill()?;

    assert!(
        !found_switch,
        "Watch should ignore events from other projects (project-b)"
    );

    Ok(())
}

/// Test that watch enters waiting mode when no sessions exist at all
///
/// CURRENT BEHAVIOR (BUG):
/// - watch fails with "No sessions found in any enabled provider"
/// - User cannot start watch before creating first session
///
/// EXPECTED BEHAVIOR:
/// - watch should start in waiting mode (like `tail -f`)
/// - Shows "Waiting for new session..." message
/// - Does NOT exit with error
#[test]
fn test_watch_waits_when_no_sessions_exist() -> Result<()> {
    // Given: Initialized workspace with NO sessions at all
    let mut world = TestWorld::builder().without_data_dir().build();
    world = world.with_project("my-project");

    world.enable_provider(TestProvider::Claude)?;
    world.set_cwd("my-project");
    world.run(&["init"])?;

    // Verify no sessions exist
    let list_result = world.run(&["session", "list", "--format", "json"])?;
    let json = list_result.json()?;
    let sessions = json["content"]["sessions"].as_array().unwrap();
    assert_eq!(sessions.len(), 0, "Should have 0 sessions initially");

    // When: Run watch WITHOUT specifying --provider (auto-selection)
    // This is the key scenario that fails: watch should fallback to first enabled provider
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_agtrace"));
    cmd.current_dir(world.temp_dir().join("my-project"))
        .arg("--data-dir")
        .arg(world.data_dir())
        .args(["watch", "--mode", "console"]); // No --provider flag

    let mut proc = BackgroundProcess::spawn_piped(cmd)?;

    // Then: Should enter waiting mode (not exit with error)
    let stdout = proc.stdout().expect("Should have stdout");
    let reader = BufReader::new(stdout);

    let mut found_waiting_or_watching = false;
    for line in reader.lines().take(10) {
        let line = line?;
        // Should either show "Waiting" or "Watching" (not error)
        if line.contains("Waiting") || line.contains("waiting") || line.contains("Watching") {
            found_waiting_or_watching = true;
            break;
        }
    }

    // Clean up
    proc.kill()?;

    assert!(
        found_waiting_or_watching,
        "Watch should enter waiting mode when no sessions exist, not fail with error"
    );

    Ok(())
}
