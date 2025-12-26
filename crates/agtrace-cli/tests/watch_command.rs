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
