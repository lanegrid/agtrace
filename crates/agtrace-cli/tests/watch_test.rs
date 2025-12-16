use agtrace_cli::streaming::{SessionWatcher, StreamEvent};
use agtrace_providers::{create_provider, LogProvider};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;

/// Test that SessionRotated event is followed by Attached event for new session
#[test]
fn test_session_rotation_emits_attached_event() {
    let temp_dir = TempDir::new().unwrap();
    let log_root = temp_dir.path().join(".claude");
    fs::create_dir_all(&log_root).unwrap();

    // Copy initial session file
    let samples_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("agtrace-providers/tests/samples");

    let session1 = log_root.join("session1.jsonl");
    fs::copy(samples_dir.join("claude_session.jsonl"), &session1).unwrap();

    // Update mtime to make it "recent" (within 5 min threshold)
    let now = filetime::FileTime::now();
    filetime::set_file_mtime(&session1, now).unwrap();

    let provider: Arc<dyn LogProvider> = Arc::from(create_provider("claude_code").unwrap());

    // Create watcher
    let watcher = SessionWatcher::new(log_root.clone(), provider.clone(), None, None).unwrap();

    let rx = watcher.receiver();

    // Verify initial Attached event
    let event = rx
        .recv_timeout(Duration::from_secs(2))
        .expect("Should receive Attached event");

    match event {
        StreamEvent::Attached { path, .. } => {
            assert_eq!(path, session1);
        }
        _ => panic!("Expected Attached event, got: {:?}", event),
    }

    // After Attached, should receive Update with initial events
    let event = rx
        .recv_timeout(Duration::from_secs(2))
        .expect("Should receive Update event after Attached");

    match event {
        StreamEvent::Update(_) => {
            // Expected - initial snapshot
        }
        other => panic!("Expected Update event after Attached, got: {:?}", other),
    }

    // Wait a bit to ensure watcher is stable
    std::thread::sleep(Duration::from_millis(500));

    // Create a newer session file to trigger rotation
    let session2 = log_root.join("session2.jsonl");
    std::thread::sleep(Duration::from_millis(100)); // Ensure newer mtime
    fs::copy(samples_dir.join("claude_agent.jsonl"), &session2).unwrap();

    // Set newer mtime for session2
    let later = filetime::FileTime::from_unix_time(now.unix_seconds() + 10, 0);
    filetime::set_file_mtime(&session2, later).unwrap();

    // Should receive SessionRotated event
    let event = rx
        .recv_timeout(Duration::from_secs(3))
        .expect("Should receive SessionRotated event");

    match event {
        StreamEvent::SessionRotated { old_path, new_path } => {
            assert_eq!(old_path, session1);
            assert_eq!(new_path, session2);
        }
        _ => panic!("Expected SessionRotated event, got: {:?}", event),
    }

    // After rotation, should also receive Attached event for the new session
    let event = rx
        .recv_timeout(Duration::from_secs(2))
        .expect("Should receive Attached event for new session after rotation");

    match event {
        StreamEvent::Attached { path, .. } => {
            assert_eq!(
                path, session2,
                "Should receive Attached event for new session"
            );
        }
        other => {
            panic!("Expected Attached event after rotation, got: {:?}", other);
        }
    }

    // After new Attached, should receive Update with initial events
    let event = rx
        .recv_timeout(Duration::from_secs(2))
        .expect("Should receive Update event after new Attached");

    match event {
        StreamEvent::Update(_) => {
            // Expected - initial snapshot for new session
        }
        other => panic!("Expected Update event after new Attached, got: {:?}", other),
    }
}

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
        StreamEvent::Waiting { .. } => {}
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
        StreamEvent::Attached { path, .. } => {
            assert_eq!(path, session_file);
        }
        other => panic!("Expected Attached event, got: {:?}", other),
    }

    // Should also receive an Update event with existing events from the file
    let event = rx.recv_timeout(Duration::from_secs(2));

    match event {
        Ok(StreamEvent::Update(update)) => {
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
