mod common;
use common::TestFixture;

use std::fs;

/// Test that init handles scan errors gracefully and provides useful feedback
#[test]
fn test_init_with_scan_error_provides_helpful_output() {
    let fixture = TestFixture::new();

    // Setup a provider with non-existent log root to cause scan error
    let mut cmd = fixture.command();
    let non_existent_path = fixture.log_root().join("non_existent");

    let output = cmd
        .arg("provider")
        .arg("set")
        .arg("claude_code")
        .arg("--log-root")
        .arg(&non_existent_path)
        .arg("--enable")
        .output()
        .expect("Failed to run provider set");

    assert!(output.status.success(), "provider set should succeed");

    // Run init - should handle missing log_root gracefully
    let mut cmd = fixture.command();
    let output = cmd
        .arg("init")
        .arg("--all-projects")
        .output()
        .expect("Failed to run init");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Init should succeed even with scan errors
    assert!(
        output.status.success(),
        "init should succeed despite scan errors.\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );

    // Should show warning about scan error
    assert!(
        stdout.contains("Warning") || stdout.contains("does not exist"),
        "Should display warning about scan error, got:\n{}",
        stdout
    );

    // Database should still be created
    let db_path = fixture.data_dir().join("agtrace.db");
    assert!(
        db_path.exists(),
        "Database should be created even if scan fails"
    );
}

/// Test that init with valid but empty log directory succeeds
#[test]
fn test_init_with_empty_log_directory() {
    let fixture = TestFixture::new();

    // Setup provider with empty log root (exists but has no sessions)
    fixture
        .setup_provider("claude_code")
        .expect("Failed to setup provider");

    // Run init
    let mut cmd = fixture.command();
    let output = cmd
        .arg("init")
        .arg("--all-projects")
        .output()
        .expect("Failed to run init");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should succeed
    assert!(
        output.status.success(),
        "init should succeed with empty log directory.\nstdout: {}",
        stdout
    );

    // Should indicate no sessions found
    assert!(
        stdout.contains("No sessions found") || stdout.contains("0 sessions"),
        "Should indicate no sessions found, got:\n{}",
        stdout
    );

    // Should provide helpful tips
    assert!(
        stdout.contains("Tips") || stdout.contains("Check provider"),
        "Should provide tips for next steps, got:\n{}",
        stdout
    );
}

/// Test that init with corrupted session file shows diagnostic suggestion
#[test]
fn test_init_with_corrupted_session_file() {
    let fixture = TestFixture::new();

    fixture
        .setup_provider("claude_code")
        .expect("Failed to setup provider");

    // Create a corrupted session file
    let corrupted_file = fixture.log_root().join("corrupted.jsonl");
    fs::write(&corrupted_file, "{ this is not valid json }\n").expect("Failed to write file");

    // Run init
    let mut cmd = fixture.command();
    let output = cmd
        .arg("init")
        .arg("--all-projects")
        .output()
        .expect("Failed to run init");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should succeed (graceful handling)
    assert!(
        output.status.success(),
        "init should succeed even with corrupted files.\nstdout: {}",
        stdout
    );

    // Should mention compatibility issues or suggest doctor command
    assert!(
        stdout.contains("doctor") || stdout.contains("compatibility"),
        "Should suggest running doctor for issues, got:\n{}",
        stdout
    );
}
