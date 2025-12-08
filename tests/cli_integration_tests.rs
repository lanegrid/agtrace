use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;
use std::fs;
use std::path::Path;

#[test]
fn test_cli_version() {
    let mut cmd = Command::cargo_bin("agtrace").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("agtrace"));
}

#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("agtrace").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("import"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("show"));
}

#[test]
fn test_import_claude_dry_run() {
    let mut cmd = Command::cargo_bin("agtrace").unwrap();
    let temp_dir = TempDir::new().unwrap();

    cmd.arg("import")
        .arg("--source").arg("claude")
        .arg("--root").arg("tests/fixtures/claude")
        .arg("--data-dir").arg(temp_dir.path())
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("session").or(predicate::str::contains("event")));
}

#[test]
fn test_import_claude_actual() {
    let mut cmd = Command::cargo_bin("agtrace").unwrap();
    let temp_dir = TempDir::new().unwrap();

    cmd.arg("import")
        .arg("--source").arg("claude")
        .arg("--root").arg("tests/fixtures/claude")
        .arg("--data-dir").arg(temp_dir.path())
        .arg("--all-projects")
        .assert()
        .success();

    // Verify events were written
    let data_dir = temp_dir.path();
    assert!(data_dir.join("projects").exists(), "projects directory should be created");
}

#[test]
fn test_import_codex() {
    let mut cmd = Command::cargo_bin("agtrace").unwrap();
    let temp_dir = TempDir::new().unwrap();

    cmd.arg("import")
        .arg("--source").arg("codex")
        .arg("--root").arg("tests/fixtures/codex")
        .arg("--data-dir").arg(temp_dir.path())
        .arg("--all-projects")
        .assert()
        .success();
}

#[test]
fn test_import_gemini() {
    let mut cmd = Command::cargo_bin("agtrace").unwrap();
    let temp_dir = TempDir::new().unwrap();

    cmd.arg("import")
        .arg("--source").arg("gemini")
        .arg("--root").arg("tests/fixtures/gemini")
        .arg("--data-dir").arg(temp_dir.path())
        .arg("--all-projects")
        .assert()
        .success();
}

#[test]
fn test_list_sessions() {
    let temp_dir = TempDir::new().unwrap();

    // First import some data
    let mut import_cmd = Command::cargo_bin("agtrace").unwrap();
    import_cmd
        .arg("import")
        .arg("--source").arg("claude")
        .arg("--root").arg("tests/fixtures/claude")
        .arg("--data-dir").arg(temp_dir.path())
        .arg("--all-projects")
        .assert()
        .success();

    // Now list sessions
    let mut list_cmd = Command::cargo_bin("agtrace").unwrap();
    list_cmd
        .arg("list")
        .arg("--data-dir").arg(temp_dir.path())
        .arg("--all-projects")
        .assert()
        .success()
        .stdout(predicate::str::contains("SESSION").or(predicate::str::contains("session")));
}

#[test]
fn test_list_sessions_json_format() {
    let temp_dir = TempDir::new().unwrap();

    // Import data
    let mut import_cmd = Command::cargo_bin("agtrace").unwrap();
    import_cmd
        .arg("import")
        .arg("--source").arg("claude")
        .arg("--root").arg("tests/fixtures/claude")
        .arg("--data-dir").arg(temp_dir.path())
        .arg("--all-projects")
        .assert()
        .success();

    // List with JSON format
    let mut list_cmd = Command::cargo_bin("agtrace").unwrap();
    list_cmd
        .arg("list")
        .arg("--data-dir").arg(temp_dir.path())
        .arg("--format").arg("json")
        .arg("--all-projects")
        .assert()
        .success()
        .stdout(predicate::str::contains("["))
        .stdout(predicate::str::contains("session_id"));
}

#[test]
fn test_show_session() {
    let temp_dir = TempDir::new().unwrap();

    // Import data
    let mut import_cmd = Command::cargo_bin("agtrace").unwrap();
    import_cmd
        .arg("import")
        .arg("--source").arg("claude")
        .arg("--root").arg("tests/fixtures/claude")
        .arg("--data-dir").arg(temp_dir.path())
        .arg("--all-projects")
        .assert()
        .success();

    // Get session ID from list
    let mut list_cmd = Command::cargo_bin("agtrace").unwrap();
    let list_output = list_cmd
        .arg("list")
        .arg("--data-dir").arg(temp_dir.path())
        .arg("--format").arg("json")
        .arg("--all-projects")
        .output()
        .unwrap();

    let sessions: serde_json::Value = serde_json::from_slice(&list_output.stdout).unwrap();
    let session_id = sessions[0]["session_id"].as_str().unwrap();

    // Show session details
    let mut show_cmd = Command::cargo_bin("agtrace").unwrap();
    show_cmd
        .arg("show")
        .arg(session_id)
        .arg("--data-dir").arg(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("user_message")
            .or(predicate::str::contains("assistant_message")));
}

#[test]
fn test_show_session_with_filters() {
    let temp_dir = TempDir::new().unwrap();

    // Import data
    let mut import_cmd = Command::cargo_bin("agtrace").unwrap();
    import_cmd
        .arg("import")
        .arg("--source").arg("claude")
        .arg("--root").arg("tests/fixtures/claude")
        .arg("--data-dir").arg(temp_dir.path())
        .arg("--all-projects")
        .assert()
        .success();

    // Get session ID
    let mut list_cmd = Command::cargo_bin("agtrace").unwrap();
    let list_output = list_cmd
        .arg("list")
        .arg("--data-dir").arg(temp_dir.path())
        .arg("--format").arg("json")
        .arg("--all-projects")
        .output()
        .unwrap();

    let sessions: serde_json::Value = serde_json::from_slice(&list_output.stdout).unwrap();
    let session_id = sessions[0]["session_id"].as_str().unwrap();

    // Show with no-reasoning filter
    let mut show_cmd = Command::cargo_bin("agtrace").unwrap();
    show_cmd
        .arg("show")
        .arg(session_id)
        .arg("--data-dir").arg(temp_dir.path())
        .arg("--no-reasoning")
        .assert()
        .success();
}

#[test]
fn test_import_missing_source() {
    let mut cmd = Command::cargo_bin("agtrace").unwrap();
    let temp_dir = TempDir::new().unwrap();

    cmd.arg("import")
        .arg("--root").arg("tests/fixtures/claude")
        .arg("--data-dir").arg(temp_dir.path())
        .assert()
        .failure();
}

#[test]
fn test_import_invalid_source() {
    let mut cmd = Command::cargo_bin("agtrace").unwrap();
    let temp_dir = TempDir::new().unwrap();

    cmd.arg("import")
        .arg("--source").arg("invalid_source")
        .arg("--root").arg("tests/fixtures/claude")
        .arg("--data-dir").arg(temp_dir.path())
        .assert()
        .failure();
}
