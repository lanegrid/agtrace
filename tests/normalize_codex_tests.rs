use agtrace::model::*;
use agtrace::providers::codex::*;
use std::path::Path;

#[test]
fn test_parse_codex_simple_session() {
    let path = Path::new("tests/fixtures/codex/simple_session.jsonl");
    let session_id = "codex-test-session";
    let events = normalize_codex_file(path, session_id, None).unwrap();

    // Should produce 5 events (user, reasoning, tool_call, tool_result, assistant)
    assert_eq!(events.len(), 5, "Expected 5 events from Codex fixture");

    // First event should be user_message
    let user_event = &events[0];
    assert_eq!(user_event.source, Source::Codex);
    assert_eq!(user_event.event_type, EventType::UserMessage);
    assert_eq!(user_event.role, Some(Role::User));
    assert_eq!(
        user_event.parent_event_id, None,
        "User message should have no parent"
    );
    assert_eq!(user_event.session_id, Some(session_id.to_string()));
    assert!(user_event.text.as_ref().unwrap().contains("summarize"));
}

#[test]
fn test_codex_reasoning_event() {
    let path = Path::new("tests/fixtures/codex/simple_session.jsonl");
    let events = normalize_codex_file(path, "test-session", None).unwrap();

    // Second event should be reasoning
    let reasoning_event = &events[1];
    assert_eq!(reasoning_event.event_type, EventType::Reasoning);
    assert_eq!(reasoning_event.role, Some(Role::Assistant));
    assert!(reasoning_event.parent_event_id.is_some());
    assert!(reasoning_event.text.as_ref().unwrap().contains("README"));
}

#[test]
fn test_codex_tool_call_and_result() {
    let path = Path::new("tests/fixtures/codex/simple_session.jsonl");
    let events = normalize_codex_file(path, "test-session", None).unwrap();

    // Find tool_call (should be 3rd event)
    let tool_call = &events[2];
    assert_eq!(tool_call.event_type, EventType::ToolCall);
    assert_eq!(tool_call.tool_name, Some("shell".to_string()));
    assert_eq!(tool_call.tool_call_id, Some("call_001".to_string()));
    assert_eq!(tool_call.channel, Some(Channel::Terminal));

    // Find tool_result (should be 4th event)
    let tool_result = &events[3];
    assert_eq!(tool_result.event_type, EventType::ToolResult);
    assert_eq!(tool_result.tool_call_id, Some("call_001".to_string()));
    assert!(tool_result.tool_status.is_some());
    assert_eq!(tool_result.tool_status, Some(ToolStatus::Success));

    // Verify event_ids are different but tool_call_id is the same
    assert_ne!(
        tool_call.event_id, tool_result.event_id,
        "tool_call and tool_result should have different event_ids"
    );
    assert_eq!(
        tool_call.tool_call_id, tool_result.tool_call_id,
        "tool_call and tool_result should share the same tool_call_id"
    );

    // v1.5: tool_call role should be Assistant
    assert_eq!(
        tool_call.role,
        Some(Role::Assistant),
        "tool_call role should be Assistant"
    );

    // v1.5: tool_result role should be Tool (not Assistant)
    assert_eq!(
        tool_result.role,
        Some(Role::Tool),
        "tool_result role should be Tool, not Assistant"
    );
}

#[test]
fn test_codex_project_hash() {
    let path = Path::new("tests/fixtures/codex/simple_session.jsonl");
    let events = normalize_codex_file(path, "test-session", None).unwrap();

    // All events should have same project_hash from cwd
    let first_hash = &events[0].project_hash;
    for event in &events {
        assert_eq!(&event.project_hash, first_hash);
        assert_eq!(event.project_root, Some("/test/project".to_string()));
    }
}

#[test]
fn test_codex_parent_event_chain() {
    let path = Path::new("tests/fixtures/codex/simple_session.jsonl");
    let events = normalize_codex_file(path, "test-session", None).unwrap();

    let user_msg = &events[0];
    assert_eq!(user_msg.event_type, EventType::UserMessage);
    assert_eq!(user_msg.parent_event_id, None);

    // All other events should point to user message as parent
    for event in &events[1..] {
        assert_eq!(event.parent_event_id, user_msg.event_id);
    }
}

#[test]
fn test_codex_token_usage() {
    let path = Path::new("tests/fixtures/codex/simple_session.jsonl");
    let events = normalize_codex_file(path, "test-session", None).unwrap();

    // Tool result event should have token info from last_token_usage
    let tool_result = events
        .iter()
        .find(|e| e.event_type == EventType::ToolResult)
        .expect("Should have tool_result");

    assert!(tool_result.tokens_input.is_some());
    assert_eq!(tool_result.tokens_input, Some(500));
    assert_eq!(tool_result.tokens_output, Some(200));
    assert_eq!(tool_result.tokens_total, Some(700));
}

#[test]
fn test_codex_assistant_message() {
    let path = Path::new("tests/fixtures/codex/simple_session.jsonl");
    let events = normalize_codex_file(path, "test-session", None).unwrap();

    let assistant_msg = events
        .iter()
        .find(|e| e.event_type == EventType::AssistantMessage)
        .expect("Should have assistant message");

    assert_eq!(assistant_msg.role, Some(Role::Assistant));
    assert_eq!(assistant_msg.channel, Some(Channel::Chat));
    assert!(assistant_msg.text.is_some());
    assert!(assistant_msg.text.as_ref().unwrap().contains("agtrace"));
}

#[test]
fn test_codex_schema_version() {
    let path = Path::new("tests/fixtures/codex/simple_session.jsonl");
    let events = normalize_codex_file(path, "test-session", None).unwrap();

    for event in &events {
        assert_eq!(event.schema_version, AgentEventV1::SCHEMA_VERSION);
    }
}
