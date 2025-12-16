use assert_cmd::Command;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Test fixture that sets up a temporary agtrace environment
struct TestFixture {
    _temp_dir: TempDir,
    data_dir: PathBuf,
    log_root: PathBuf,
}

impl TestFixture {
    fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let data_dir = temp_dir.path().join(".agtrace");
        // Use .claude subdirectory to match provider detection logic in session_loader
        let log_root = temp_dir.path().join(".claude");

        fs::create_dir_all(&data_dir).expect("Failed to create data dir");
        fs::create_dir_all(&log_root).expect("Failed to create log dir");

        Self {
            _temp_dir: temp_dir,
            data_dir,
            log_root,
        }
    }

    fn data_dir(&self) -> &PathBuf {
        &self.data_dir
    }

    fn log_root(&self) -> &PathBuf {
        &self.log_root
    }

    /// Copy a sample session file to the test log directory
    fn copy_sample_file(&self, sample_name: &str, dest_name: &str) -> anyhow::Result<()> {
        let samples_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("agtrace-providers/tests/samples");

        let source = samples_dir.join(sample_name);
        let dest = self.log_root.join(dest_name);

        fs::copy(source, dest)?;
        Ok(())
    }

    /// Run agtrace command with this fixture's data directory
    fn command(&self) -> Command {
        let mut cmd = Command::cargo_bin("agtrace").expect("Failed to find agtrace binary");
        cmd.arg("--data-dir").arg(self.data_dir());
        cmd
    }

    /// Setup a provider using provider set command
    fn setup_provider(&self, provider_name: &str) -> anyhow::Result<()> {
        let mut cmd = self.command();
        let output = cmd
            .arg("provider")
            .arg("set")
            .arg(provider_name)
            .arg("--log-root")
            .arg(self.log_root())
            .arg("--enable")
            .output()?;

        if !output.status.success() {
            anyhow::bail!(
                "provider set failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(())
    }

    /// Run index update
    fn index_update(&self) -> anyhow::Result<()> {
        let mut cmd = self.command();
        let output = cmd
            .arg("index")
            .arg("update")
            .arg("--all-projects")
            .arg("--verbose")
            .output()?;

        if !output.status.success() {
            anyhow::bail!(
                "index update failed: {}\nstdout: {}",
                String::from_utf8_lossy(&output.stderr),
                String::from_utf8_lossy(&output.stdout)
            );
        }
        Ok(())
    }
}

#[test]
fn test_init_full_workflow() {
    let fixture = TestFixture::new();

    // Step 1: Copy sample file
    fixture
        .copy_sample_file("claude_session.jsonl", "session1.jsonl")
        .expect("Failed to copy sample file");

    // Step 2: Manually configure provider using provider set
    fixture
        .setup_provider("claude_code")
        .expect("Failed to setup provider");

    // Verify config was created
    let config_path = fixture.data_dir().join("config.toml");
    assert!(
        config_path.exists(),
        "Config file should be created at {}",
        config_path.display()
    );

    // Step 3: Run init to scan and index
    let mut cmd = fixture.command();
    let output = cmd
        .arg("init")
        .arg("--all-projects")
        .output()
        .expect("Failed to run init");

    assert!(
        output.status.success(),
        "init command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Step 4: Verify database was created
    let db_path = fixture.data_dir().join("agtrace.db");
    assert!(
        db_path.exists(),
        "Database should be created at {}",
        db_path.display()
    );

    // Step 5: Run session list to verify indexing worked
    let mut cmd = fixture.command();
    let output = cmd
        .arg("session")
        .arg("list")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to run session list");

    assert!(
        output.status.success(),
        "session list failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let sessions: serde_json::Value =
        serde_json::from_str(&stdout).expect("Failed to parse JSON output");

    assert!(sessions.is_array(), "Expected JSON array, got: {}", stdout);

    let sessions_array = sessions.as_array().unwrap();
    assert!(
        !sessions_array.is_empty(),
        "Expected at least one session to be indexed"
    );
}

#[test]
fn test_index_scan_and_query() {
    let fixture = TestFixture::new();

    // Setup: Configure provider and copy sample files
    fixture
        .setup_provider("claude_code")
        .expect("Failed to setup provider");

    fixture
        .copy_sample_file("claude_session.jsonl", "session1.jsonl")
        .expect("Failed to copy sample 1");

    fixture
        .copy_sample_file("claude_agent.jsonl", "session2.jsonl")
        .expect("Failed to copy sample 2");

    // Step 1: Run index update
    fixture.index_update().expect("Failed to run index update");

    // Step 2: Query sessions and verify consistency
    let mut cmd = fixture.command();
    let output = cmd
        .arg("session")
        .arg("list")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to run session list");

    assert!(
        output.status.success(),
        "session list failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let sessions: serde_json::Value =
        serde_json::from_str(&stdout).expect("Failed to parse JSON output");

    assert!(
        sessions.is_array(),
        "Expected JSON array of sessions, got: {}",
        stdout
    );

    let sessions_array = sessions.as_array().unwrap();
    assert!(
        sessions_array.len() >= 1,
        "Expected at least 1 session, found {}",
        sessions_array.len()
    );

    // Step 3: Verify we can show individual sessions
    for session in sessions_array {
        let session_id = session["id"]
            .as_str()
            .expect("Session should have string id");

        let mut cmd = fixture.command();
        let output = cmd
            .arg("session")
            .arg("show")
            .arg(session_id)
            .arg("--json")
            .output()
            .expect("Failed to run session show");

        assert!(
            output.status.success(),
            "session show {} failed: {}",
            session_id,
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn test_session_show_filtering() {
    let fixture = TestFixture::new();

    // Setup
    fixture
        .setup_provider("claude_code")
        .expect("Failed to setup provider");

    fixture
        .copy_sample_file("claude_session.jsonl", "session1.jsonl")
        .expect("Failed to copy sample file");

    // Index the session
    fixture.index_update().expect("Failed to index");

    // Get session ID
    let mut cmd = fixture.command();
    let output = cmd
        .arg("session")
        .arg("list")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to run session list");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let sessions: serde_json::Value = serde_json::from_str(&stdout).expect("Failed to parse JSON");
    let sessions_array = sessions.as_array().expect("Expected array");

    assert!(!sessions_array.is_empty(), "Expected at least one session");

    let session_id = sessions_array[0]["id"]
        .as_str()
        .expect("Expected session id");

    // Test 1: Show with --json (baseline)
    let mut cmd = fixture.command();
    let output = cmd
        .arg("session")
        .arg("show")
        .arg(session_id)
        .arg("--json")
        .output()
        .expect("Failed to run session show");

    assert!(output.status.success());
    let all_events: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&output.stdout))
            .expect("Failed to parse JSON");
    let all_events_array = all_events.as_array().expect("Expected array");
    let total_events = all_events_array.len();

    assert!(total_events > 0, "Expected at least one event");

    // Test 2: Show with --hide to filter out specific event types
    let mut cmd = fixture.command();
    let output = cmd
        .arg("session")
        .arg("show")
        .arg(session_id)
        .arg("--json")
        .arg("--hide")
        .arg("text")
        .output()
        .expect("Failed to run session show with --hide");

    assert!(output.status.success());
    let filtered_events: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&output.stdout))
            .expect("Failed to parse JSON");
    let filtered_array = filtered_events.as_array().expect("Expected array");

    // Verify no "text" events remain (if there were any to filter)
    for event in filtered_array {
        let payload = &event["payload"];
        if let Some(payload_obj) = payload.as_object() {
            assert!(
                !payload_obj.contains_key("Text"),
                "Text events should be filtered out"
            );
        }
    }

    // If filtering reduced the count, that's a bonus (but not required if no Text events existed)
    assert!(
        filtered_array.len() <= total_events,
        "Filtered count should not exceed total"
    );

    // Test 3: Show with --only to include only specific event types
    let mut cmd = fixture.command();
    let output = cmd
        .arg("session")
        .arg("show")
        .arg(session_id)
        .arg("--json")
        .arg("--only")
        .arg("tool_use")
        .output()
        .expect("Failed to run session show with --only");

    assert!(output.status.success());
    let only_events: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&output.stdout))
            .expect("Failed to parse JSON");
    let only_array = only_events.as_array().expect("Expected array");

    // Verify only "tool_use" events remain (if any exist)
    for event in only_array {
        let payload = &event["payload"];
        if let Some(payload_obj) = payload.as_object() {
            assert!(
                payload_obj.contains_key("ToolUse"),
                "Only ToolUse events should remain"
            );
        }
    }
}
