use agtrace::model::*;
use agtrace::normalize::gemini::*;
use std::path::Path;

#[test]
fn test_parse_gemini_simple_session() {
    let path = Path::new("tests/fixtures/gemini/simple_session.json");
    let events = normalize_gemini_file(path).unwrap();

    // Should produce multiple events from messages array
    // At minimum: 1 user, 1 assistant, 1 reasoning, 1 tool_call, 1 info/meta
    assert!(
        events.len() >= 3,
        "Expected at least 3 events from Gemini fixture"
    );

    // First message should be user_message
    let user_events: Vec<_> = events
        .iter()
        .filter(|e| e.event_type == EventType::UserMessage)
        .collect();
    assert!(
        !user_events.is_empty(),
        "Should have at least one user message"
    );

    let user_event = user_events[0];
    assert_eq!(user_event.source, Source::Gemini);
    assert_eq!(user_event.role, Some(Role::User));
    assert_eq!(user_event.parent_event_id, None);
    assert!(user_event.text.as_ref().unwrap().contains("list files"));
}

#[test]
fn test_gemini_project_hash() {
    let path = Path::new("tests/fixtures/gemini/simple_session.json");
    let events = normalize_gemini_file(path).unwrap();

    // All events should have projectHash from the session
    for event in &events {
        assert_eq!(event.project_hash, "abc123def456");
        assert_eq!(
            event.project_root, None,
            "Gemini logs don't contain project_root"
        );
    }
}

#[test]
fn test_gemini_session_id() {
    let path = Path::new("tests/fixtures/gemini/simple_session.json");
    let events = normalize_gemini_file(path).unwrap();

    // All events should have same session_id
    for event in &events {
        assert_eq!(event.session_id, Some("gemini-session-1".to_string()));
    }
}

#[test]
fn test_gemini_assistant_message() {
    let path = Path::new("tests/fixtures/gemini/simple_session.json");
    let events = normalize_gemini_file(path).unwrap();

    let assistant_msgs: Vec<_> = events
        .iter()
        .filter(|e| e.event_type == EventType::AssistantMessage)
        .collect();

    assert!(!assistant_msgs.is_empty(), "Should have assistant message");

    let assistant = assistant_msgs[0];
    assert_eq!(assistant.role, Some(Role::Assistant));
    assert_eq!(assistant.channel, Some(Channel::Chat));
    assert!(assistant.text.is_some());
}

#[test]
fn test_gemini_thoughts_as_reasoning() {
    let path = Path::new("tests/fixtures/gemini/simple_session.json");
    let events = normalize_gemini_file(path).unwrap();

    // thoughts array should be converted to reasoning events
    let reasoning_events: Vec<_> = events
        .iter()
        .filter(|e| e.event_type == EventType::Reasoning)
        .collect();

    assert!(
        !reasoning_events.is_empty(),
        "Should have reasoning events from thoughts"
    );

    let reasoning = reasoning_events[0];
    assert_eq!(reasoning.role, Some(Role::Assistant));
    assert!(reasoning.text.is_some());
    assert!(reasoning.text.as_ref().unwrap().contains("Tool selection"));
}

#[test]
fn test_gemini_tool_calls() {
    let path = Path::new("tests/fixtures/gemini/simple_session.json");
    let events = normalize_gemini_file(path).unwrap();

    let tool_calls: Vec<_> = events
        .iter()
        .filter(|e| e.event_type == EventType::ToolCall)
        .collect();

    assert!(!tool_calls.is_empty(), "Should have tool_call events");

    let tool_call = tool_calls[0];
    assert_eq!(tool_call.tool_name, Some("bash".to_string()));
    assert_eq!(tool_call.tool_call_id, Some("tool-001".to_string()));
    assert_eq!(tool_call.tool_status, Some(ToolStatus::Success));
}

#[test]
fn test_gemini_info_as_meta() {
    let path = Path::new("tests/fixtures/gemini/simple_session.json");
    let events = normalize_gemini_file(path).unwrap();

    let meta_events: Vec<_> = events
        .iter()
        .filter(|e| e.event_type == EventType::Meta || e.event_type == EventType::SystemMessage)
        .collect();

    assert!(
        !meta_events.is_empty(),
        "Should have meta/system events from info messages"
    );
}

#[test]
fn test_gemini_parent_event_chain() {
    let path = Path::new("tests/fixtures/gemini/simple_session.json");
    let events = normalize_gemini_file(path).unwrap();

    // Find user message
    let user_msg = events
        .iter()
        .find(|e| e.event_type == EventType::UserMessage)
        .expect("Should have user message");

    // All non-user events should have parent_event_id pointing to user message
    for event in &events {
        if event.event_type != EventType::UserMessage {
            assert_eq!(
                event.parent_event_id, user_msg.event_id,
                "Non-user event {:?} should have user message as parent",
                event.event_type
            );
        }
    }
}

#[test]
fn test_gemini_schema_version() {
    let path = Path::new("tests/fixtures/gemini/simple_session.json");
    let events = normalize_gemini_file(path).unwrap();

    for event in &events {
        assert_eq!(event.schema_version, AgentEventV1::SCHEMA_VERSION);
    }
}
