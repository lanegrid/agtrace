use agtrace::model::*;
use agtrace::normalize::claude::*;
use std::path::Path;

#[test]
fn test_parse_claude_simple_session() {
    let path = Path::new("tests/fixtures/claude/simple_session.jsonl");
    let events = normalize_claude_file(path, None).unwrap();

    // Should produce multiple events from the fixture
    assert!(events.len() >= 4, "Expected at least 4 events (user, thinking, tool_call, assistant)");

    // First event should be user_message
    let user_event = &events[0];
    assert_eq!(user_event.source, Source::ClaudeCode);
    assert_eq!(user_event.event_type, EventType::UserMessage);
    assert_eq!(user_event.role, Some(Role::User));
    assert_eq!(user_event.parent_event_id, None, "User message should have no parent");
    assert_eq!(user_event.session_id, Some("claude-session-1".to_string()));
    assert!(user_event.text.as_ref().unwrap().contains("list files"));

    // Second event should be reasoning (thinking)
    let reasoning_event = &events[1];
    assert_eq!(reasoning_event.event_type, EventType::Reasoning);
    assert_eq!(reasoning_event.role, Some(Role::Assistant));
    assert!(reasoning_event.parent_event_id.is_some(), "Reasoning should have parent_event_id");
    assert_eq!(reasoning_event.parent_event_id, user_event.event_id, "Reasoning parent should be user message");

    // Third event should be tool_call
    let tool_call_event = &events[2];
    assert_eq!(tool_call_event.event_type, EventType::ToolCall);
    assert_eq!(tool_call_event.tool_name, Some("Bash".to_string()));
    assert_eq!(tool_call_event.tool_call_id, Some("toolu_001".to_string()));
    assert_eq!(tool_call_event.parent_event_id, user_event.event_id);
    assert_eq!(tool_call_event.channel, Some(Channel::Terminal));
}

#[test]
fn test_claude_project_hash_from_cwd() {
    let path = Path::new("tests/fixtures/claude/simple_session.jsonl");
    let events = normalize_claude_file(path, None).unwrap();

    // All events should have the same project_hash derived from cwd
    let first_hash = &events[0].project_hash;
    for event in &events {
        assert_eq!(&event.project_hash, first_hash);
        assert_eq!(event.project_root, Some("/test/project".to_string()));
    }
}

#[test]
fn test_claude_tool_result_matching() {
    let path = Path::new("tests/fixtures/claude/simple_session.jsonl");
    let events = normalize_claude_file(path, None).unwrap();

    // Find tool_call and tool_result events
    let tool_calls: Vec<_> = events.iter()
        .filter(|e| e.event_type == EventType::ToolCall)
        .collect();

    let tool_results: Vec<_> = events.iter()
        .filter(|e| e.event_type == EventType::ToolResult)
        .collect();

    assert!(!tool_calls.is_empty(), "Should have at least one tool_call");
    assert!(!tool_results.is_empty(), "Should have at least one tool_result");

    // tool_result should have matching tool_call_id
    let call_id = &tool_calls[0].tool_call_id;
    let result_id = &tool_results[0].tool_call_id;
    assert_eq!(call_id, result_id, "Tool call and result IDs should match");

    // tool_result should have status
    assert!(tool_results[0].tool_status.is_some());
}

#[test]
fn test_claude_assistant_message_with_tokens() {
    let path = Path::new("tests/fixtures/claude/simple_session.jsonl");
    let events = normalize_claude_file(path, None).unwrap();

    // Find assistant message event(s)
    let assistant_msgs: Vec<_> = events.iter()
        .filter(|e| e.event_type == EventType::AssistantMessage)
        .collect();

    assert!(!assistant_msgs.is_empty(), "Should have at least one assistant message");

    // Check for token usage (from the fixture's last message)
    let last_assistant = assistant_msgs.last().unwrap();
    if last_assistant.tokens_input.is_some() {
        assert!(last_assistant.tokens_input.unwrap() > 0);
    }
}

#[test]
fn test_claude_parent_event_id_chain() {
    let path = Path::new("tests/fixtures/claude/simple_session.jsonl");
    let events = normalize_claude_file(path, None).unwrap();

    // Find the user_message
    let user_msg = events.iter()
        .find(|e| e.event_type == EventType::UserMessage)
        .expect("Should have a user message");

    // All non-user events should point to this user message as parent
    for event in &events {
        if event.event_type != EventType::UserMessage {
            assert_eq!(
                event.parent_event_id,
                user_msg.event_id,
                "Non-user event {:?} should have user message as parent",
                event.event_type
            );
        }
    }
}

#[test]
fn test_claude_schema_version() {
    let path = Path::new("tests/fixtures/claude/simple_session.jsonl");
    let events = normalize_claude_file(path, None).unwrap();

    for event in &events {
        assert_eq!(event.schema_version, AgentEventV1::SCHEMA_VERSION);
    }
}
