use agtrace::model::*;
use agtrace::storage::*;
use tempfile::TempDir;

fn create_test_event(
    session_id: &str,
    event_type: EventType,
    ts: &str,
) -> AgentEventV1 {
    let mut event = AgentEventV1::new(
        Source::ClaudeCode,
        "test-hash-123".to_string(),
        ts.to_string(),
        event_type,
    );
    event.session_id = Some(session_id.to_string());
    event.event_id = Some(format!("{}#{}", ts, session_id));
    event
}

#[test]
fn test_storage_save_and_load_events() {
    let temp_dir = TempDir::new().unwrap();
    let storage = Storage::new(temp_dir.path().to_path_buf());

    let events = vec![
        create_test_event("session-1", EventType::UserMessage, "2025-11-26T12:51:28.000Z"),
        create_test_event("session-1", EventType::AssistantMessage, "2025-11-26T12:51:29.000Z"),
    ];

    // Save events
    storage.save_events(&events).unwrap();

    // Load events by session_id
    let loaded = storage.load_session_events("session-1").unwrap();
    assert_eq!(loaded.len(), 2);
    assert_eq!(loaded[0].event_type, EventType::UserMessage);
    assert_eq!(loaded[1].event_type, EventType::AssistantMessage);
}

#[test]
fn test_storage_multiple_sessions() {
    let temp_dir = TempDir::new().unwrap();
    let storage = Storage::new(temp_dir.path().to_path_buf());

    let events1 = vec![
        create_test_event("session-1", EventType::UserMessage, "2025-11-26T12:51:28.000Z"),
    ];

    let events2 = vec![
        create_test_event("session-2", EventType::UserMessage, "2025-11-26T13:00:00.000Z"),
    ];

    storage.save_events(&events1).unwrap();
    storage.save_events(&events2).unwrap();

    // Load each session separately
    let loaded1 = storage.load_session_events("session-1").unwrap();
    let loaded2 = storage.load_session_events("session-2").unwrap();

    assert_eq!(loaded1.len(), 1);
    assert_eq!(loaded2.len(), 1);
    assert_eq!(loaded1[0].session_id, Some("session-1".to_string()));
    assert_eq!(loaded2[0].session_id, Some("session-2".to_string()));
}

#[test]
fn test_storage_list_sessions() {
    let temp_dir = TempDir::new().unwrap();
    let storage = Storage::new(temp_dir.path().to_path_buf());

    let events1 = vec![
        create_test_event("session-1", EventType::UserMessage, "2025-11-26T12:51:28.000Z"),
        create_test_event("session-1", EventType::AssistantMessage, "2025-11-26T12:51:29.000Z"),
    ];

    let events2 = vec![
        create_test_event("session-2", EventType::UserMessage, "2025-11-26T13:00:00.000Z"),
    ];

    storage.save_events(&events1).unwrap();
    storage.save_events(&events2).unwrap();

    // List all sessions
    let sessions = storage.list_sessions(None, None, None).unwrap();
    assert_eq!(sessions.len(), 2);

    // Verify session summaries
    let session1 = sessions.iter()
        .find(|s| s.session_id == "session-1")
        .expect("Should find session-1");
    assert_eq!(session1.event_count, 2);
    assert_eq!(session1.source, Source::ClaudeCode);
}

#[test]
fn test_storage_filter_by_source() {
    let temp_dir = TempDir::new().unwrap();
    let storage = Storage::new(temp_dir.path().to_path_buf());

    let mut event1 = create_test_event("session-1", EventType::UserMessage, "2025-11-26T12:51:28.000Z");
    event1.source = Source::ClaudeCode;

    let mut event2 = create_test_event("session-2", EventType::UserMessage, "2025-11-26T13:00:00.000Z");
    event2.source = Source::Codex;

    storage.save_events(&vec![event1]).unwrap();
    storage.save_events(&vec![event2]).unwrap();

    // Filter by source
    let claude_sessions = storage.list_sessions(None, Some(Source::ClaudeCode), None).unwrap();
    assert_eq!(claude_sessions.len(), 1);
    assert_eq!(claude_sessions[0].source, Source::ClaudeCode);

    let codex_sessions = storage.list_sessions(None, Some(Source::Codex), None).unwrap();
    assert_eq!(codex_sessions.len(), 1);
    assert_eq!(codex_sessions[0].source, Source::Codex);
}

#[test]
fn test_storage_session_summary_tokens() {
    let temp_dir = TempDir::new().unwrap();
    let storage = Storage::new(temp_dir.path().to_path_buf());

    let mut event1 = create_test_event("session-1", EventType::UserMessage, "2025-11-26T12:51:28.000Z");
    event1.tokens_input = Some(100);
    event1.tokens_output = Some(50);

    let mut event2 = create_test_event("session-1", EventType::AssistantMessage, "2025-11-26T12:51:29.000Z");
    event2.tokens_input = Some(200);
    event2.tokens_output = Some(150);

    storage.save_events(&vec![event1, event2]).unwrap();

    let sessions = storage.list_sessions(None, None, None).unwrap();
    let session = &sessions[0];

    // Should aggregate tokens
    assert_eq!(session.tokens_input_total, 300);
    assert_eq!(session.tokens_output_total, 200);
}

#[test]
fn test_storage_find_events_by_text() {
    let temp_dir = TempDir::new().unwrap();
    let storage = Storage::new(temp_dir.path().to_path_buf());

    let mut event1 = create_test_event("session-1", EventType::UserMessage, "2025-11-26T12:51:28.000Z");
    event1.text = Some("list files in directory".to_string());

    let mut event2 = create_test_event("session-1", EventType::AssistantMessage, "2025-11-26T12:51:29.000Z");
    event2.text = Some("running ls command".to_string());

    storage.save_events(&vec![event1, event2]).unwrap();

    // Search for text
    let results = storage.find_events(None, None, Some("list files"), None, None).unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0].text.as_ref().unwrap().contains("list files"));
}

#[test]
fn test_storage_load_nonexistent_session() {
    let temp_dir = TempDir::new().unwrap();
    let storage = Storage::new(temp_dir.path().to_path_buf());

    let result = storage.load_session_events("nonexistent");
    assert!(result.is_err(), "Loading nonexistent session should return error");
}
