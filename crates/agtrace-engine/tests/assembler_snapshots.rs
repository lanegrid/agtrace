use agtrace_engine::assemble_session_from_events;
use agtrace_types::AgentEvent;
use std::fs;
use std::path::Path;

// Helper to load AgentEvent[] from fixture JSON
fn load_events_from_fixture(fixture_name: &str) -> Vec<AgentEvent> {
    let path = Path::new("tests/fixtures").join(fixture_name);
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("Failed to read fixture: {}", path.display()));
    serde_json::from_str(&content)
        .unwrap_or_else(|_| panic!("Failed to parse fixture: {}", path.display()))
}

#[test]
fn test_gemini_session_assembly() {
    let events = load_events_from_fixture("gemini_events.json");

    let session = assemble_session_from_events(&events).expect("Failed to assemble session");

    assert!(!session.turns.is_empty(), "Expected at least one turn");

    insta::assert_json_snapshot!("gemini_session_assembly", session);
}

#[test]
fn test_codex_session_assembly() {
    let events = load_events_from_fixture("codex_events.json");

    let session = assemble_session_from_events(&events).expect("Failed to assemble session");

    assert!(!session.turns.is_empty(), "Expected at least one turn");

    insta::assert_json_snapshot!("codex_session_assembly", session);
}

#[test]
fn test_claude_session_assembly() {
    let events = load_events_from_fixture("claude_events.json");

    let session = assemble_session_from_events(&events).expect("Failed to assemble session");

    assert!(!session.turns.is_empty(), "Expected at least one turn");

    insta::assert_json_snapshot!("claude_session_assembly", session);
}

#[test]
fn test_session_assembly_structure() {
    use agtrace_types::*;
    use chrono::{TimeZone, Utc};
    use uuid::Uuid;

    let base_time = Utc.with_ymd_and_hms(2025, 12, 14, 0, 0, 0).unwrap();

    let trace_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").expect("valid uuid");
    let user_id = Uuid::parse_str("00000000-0000-0000-0000-000000000002").expect("valid uuid");
    let reasoning_id = Uuid::parse_str("00000000-0000-0000-0000-000000000003").expect("valid uuid");
    let tool1_id = Uuid::parse_str("00000000-0000-0000-0000-000000000004").expect("valid uuid");
    let tool_result1_id =
        Uuid::parse_str("00000000-0000-0000-0000-000000000005").expect("valid uuid");
    let message_id = Uuid::parse_str("00000000-0000-0000-0000-000000000006").expect("valid uuid");
    let token_usage_id =
        Uuid::parse_str("00000000-0000-0000-0000-000000000007").expect("valid uuid");

    let events = vec![
        AgentEvent {
            id: user_id,
            trace_id,
            parent_id: None,
            timestamp: base_time,
            stream_id: agtrace_types::StreamId::Main,
            payload: EventPayload::User(UserPayload {
                text: "Hello".to_string(),
            }),
            metadata: None,
        },
        AgentEvent {
            id: reasoning_id,
            trace_id,
            parent_id: Some(user_id),
            timestamp: base_time,
            stream_id: agtrace_types::StreamId::Main,
            payload: EventPayload::Reasoning(ReasoningPayload {
                text: "I should respond".to_string(),
            }),
            metadata: None,
        },
        AgentEvent {
            id: tool1_id,
            trace_id,
            parent_id: Some(reasoning_id),
            timestamp: base_time,
            stream_id: agtrace_types::StreamId::Main,
            payload: EventPayload::ToolCall(ToolCallPayload {
                name: "bash".to_string(),
                arguments: serde_json::json!({"command": "echo hello"}),
                provider_call_id: Some("call_1".to_string()),
            }),
            metadata: None,
        },
        AgentEvent {
            id: tool_result1_id,
            trace_id,
            parent_id: Some(tool1_id),
            timestamp: base_time,
            stream_id: agtrace_types::StreamId::Main,
            payload: EventPayload::ToolResult(ToolResultPayload {
                output: "hello".to_string(),
                tool_call_id: tool1_id,
                is_error: false,
            }),
            metadata: None,
        },
        AgentEvent {
            id: message_id,
            trace_id,
            parent_id: Some(tool_result1_id),
            timestamp: base_time,
            stream_id: agtrace_types::StreamId::Main,
            payload: EventPayload::Message(MessagePayload {
                text: "Done!".to_string(),
            }),
            metadata: None,
        },
        AgentEvent {
            id: token_usage_id,
            trace_id,
            parent_id: Some(message_id),
            timestamp: base_time,
            stream_id: agtrace_types::StreamId::Main,
            payload: EventPayload::TokenUsage(TokenUsagePayload {
                input_tokens: 100,
                output_tokens: 50,
                total_tokens: 150,
                details: None,
            }),
            metadata: None,
        },
    ];

    let session = assemble_session_from_events(&events).expect("Failed to assemble session");

    assert_eq!(session.turns.len(), 1);
    let turn = &session.turns[0];
    assert_eq!(turn.user.content.text, "Hello");
    assert_eq!(turn.steps.len(), 1);

    let step = &turn.steps[0];
    assert!(step.reasoning.is_some());
    assert!(step.message.is_some());
    assert_eq!(step.tools.len(), 1);
    assert_eq!(step.tools[0].call.content.name, "bash");
    assert!(step.tools[0].result.is_some());
    assert!(step.usage.is_some());
    assert_eq!(step.usage.as_ref().unwrap().total_tokens, 150);

    insta::assert_json_snapshot!("session_assembly_structure", session);
}
