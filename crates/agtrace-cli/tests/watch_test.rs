use agtrace_providers::{create_provider, LogProvider};
use agtrace_runtime::{SessionWatcher, WatchEvent};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;

// TODO: Add test for session rotation (SessionRotated event)
// Current sample files (claude_session.jsonl and claude_agent.jsonl) share the same sessionId,
// so they are treated as main + sidechain of the same session rather than separate sessions.
// To properly test session rotation, we need sample files with different sessionIds.
// Session completeness (multi-file sessions) is already tested in session_completeness_test.rs

/// Test that watcher sends Update event when transitioning from waiting mode to active file
#[test]
fn test_waiting_mode_to_active_file_sends_update() {
    let temp_dir = TempDir::new().unwrap();
    let log_root = temp_dir.path().join(".claude");
    fs::create_dir_all(&log_root).unwrap();

    let provider: Arc<dyn LogProvider> = Arc::from(create_provider("claude_code").unwrap());

    // Create watcher with empty directory (waiting mode)
    let watcher = SessionWatcher::new(log_root.clone(), provider.clone(), None, None).unwrap();

    let rx = watcher.receiver();

    // Should receive Waiting event since no files exist
    let event = rx
        .recv_timeout(Duration::from_secs(2))
        .expect("Should receive Waiting event");

    match event {
        WatchEvent::Waiting { .. } => {}
        other => panic!("Expected Waiting event, got: {:?}", other),
    }

    // Now create a session file with existing events
    let samples_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("agtrace-providers/tests/samples");

    let session_file = log_root.join("new_session.jsonl");
    fs::copy(samples_dir.join("claude_session.jsonl"), &session_file).unwrap();

    // Make it recent
    let now = filetime::FileTime::now();
    filetime::set_file_mtime(&session_file, now).unwrap();

    // Watcher should detect the new file and attach
    let event = rx
        .recv_timeout(Duration::from_secs(3))
        .expect("Should receive Attached event after file creation");

    match event {
        WatchEvent::Attached { path, .. } => {
            assert_eq!(path, session_file);
        }
        other => panic!("Expected Attached event, got: {:?}", other),
    }

    // Should also receive an Update event with existing events from the file
    let event = rx.recv_timeout(Duration::from_secs(2));

    match event {
        Ok(WatchEvent::Update(update)) => {
            assert!(
                !update.new_events.is_empty(),
                "Should receive Update with existing events from newly attached file"
            );
        }
        Ok(other) => {
            panic!(
                "Expected Update event after attaching to file with existing events, got: {:?}",
                other
            );
        }
        Err(_) => {
            panic!(
                "No Update event received after attaching to file with existing events. \
                 The watcher attached to the file but didn't send existing events."
            );
        }
    }
}
