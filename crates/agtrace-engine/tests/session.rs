use agtrace_engine::assemble_session;
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

    let session = assemble_session(&events).expect("Failed to assemble session");

    assert!(!session.turns.is_empty(), "Expected at least one turn");

    insta::assert_json_snapshot!("gemini_session_assembly", session);
}

#[test]
fn test_codex_session_assembly() {
    let events = load_events_from_fixture("codex_events.json");

    let session = assemble_session(&events).expect("Failed to assemble session");

    assert!(!session.turns.is_empty(), "Expected at least one turn");

    insta::assert_json_snapshot!("codex_session_assembly", session);
}

#[test]
fn test_claude_session_assembly() {
    let events = load_events_from_fixture("claude_events.json");

    let session = assemble_session(&events).expect("Failed to assemble session");

    assert!(!session.turns.is_empty(), "Expected at least one turn");

    insta::assert_json_snapshot!("claude_session_assembly", session);
}

#[test]
fn test_session_assembly_structure() {
    use agtrace_types::*;
    use chrono::{TimeZone, Utc};
    use uuid::Uuid;

    let base_time = Utc.with_ymd_and_hms(2025, 12, 14, 0, 0, 0).unwrap();

    let session_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").expect("valid uuid");
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
            session_id,
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
            session_id,
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
            session_id,
            parent_id: Some(reasoning_id),
            timestamp: base_time,
            stream_id: agtrace_types::StreamId::Main,
            payload: EventPayload::ToolCall(ToolCallPayload::from_raw(
                "bash".to_string(),
                serde_json::json!({"command": "echo hello"}),
                Some("call_1".to_string()),
            )),
            metadata: None,
        },
        AgentEvent {
            id: tool_result1_id,
            session_id,
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
            session_id,
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
            session_id,
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

    let session = assemble_session(&events).expect("Failed to assemble session");

    assert_eq!(session.turns.len(), 1);
    let turn = &session.turns[0];
    assert_eq!(turn.user.content.text, "Hello");
    assert_eq!(turn.steps.len(), 1);

    let step = &turn.steps[0];
    assert!(step.reasoning.is_some());
    assert!(step.message.is_some());
    assert_eq!(step.tools.len(), 1);
    assert_eq!(step.tools[0].call.content.name(), "bash");
    assert!(step.tools[0].result.is_some());
    assert!(step.usage.is_some());
    assert_eq!(step.usage.as_ref().unwrap().total_tokens, 150);

    insta::assert_json_snapshot!("session_assembly_structure", session);
}

#[test]
fn test_step_status_determination() {
    use agtrace_engine::session::types::StepStatus;
    use agtrace_types::*;
    use chrono::{TimeZone, Utc};
    use uuid::Uuid;

    let base_time = Utc.with_ymd_and_hms(2025, 12, 22, 0, 0, 0).unwrap();
    let session_id = Uuid::new_v4();

    // Case 1: Step with message -> Done
    let user1_id = Uuid::new_v4();
    let msg1_id = Uuid::new_v4();

    // Case 2: Step with tool call only (no result) -> InProgress
    let user2_id = Uuid::new_v4();
    let tool2_id = Uuid::new_v4();

    // Case 3: Step with tool call and result -> Done
    let user3_id = Uuid::new_v4();
    let tool3_id = Uuid::new_v4();
    let result3_id = Uuid::new_v4();

    // Case 4: Step with reasoning only -> InProgress
    let user4_id = Uuid::new_v4();
    let reasoning4_id = Uuid::new_v4();

    // Case 5: Step with tool error -> Failed
    let user5_id = Uuid::new_v4();
    let tool5_id = Uuid::new_v4();
    let result5_id = Uuid::new_v4();

    let events = vec![
        // Turn 1: Message -> Done
        AgentEvent {
            id: user1_id,
            session_id,
            parent_id: None,
            timestamp: base_time,
            stream_id: StreamId::Main,
            payload: EventPayload::User(UserPayload {
                text: "Test 1".to_string(),
            }),
            metadata: None,
        },
        AgentEvent {
            id: msg1_id,
            session_id,
            parent_id: Some(user1_id),
            timestamp: base_time,
            stream_id: StreamId::Main,
            payload: EventPayload::Message(MessagePayload {
                text: "Response".to_string(),
            }),
            metadata: None,
        },
        // Turn 2: Tool without result -> InProgress
        AgentEvent {
            id: user2_id,
            session_id,
            parent_id: Some(msg1_id),
            timestamp: base_time,
            stream_id: StreamId::Main,
            payload: EventPayload::User(UserPayload {
                text: "Test 2".to_string(),
            }),
            metadata: None,
        },
        AgentEvent {
            id: tool2_id,
            session_id,
            parent_id: Some(user2_id),
            timestamp: base_time,
            stream_id: StreamId::Main,
            payload: EventPayload::ToolCall(ToolCallPayload::from_raw(
                "bash".to_string(),
                serde_json::json!({"command": "ls"}),
                Some("call_2".to_string()),
            )),
            metadata: None,
        },
        // Turn 3: Tool with result -> Done
        AgentEvent {
            id: user3_id,
            session_id,
            parent_id: Some(tool2_id),
            timestamp: base_time,
            stream_id: StreamId::Main,
            payload: EventPayload::User(UserPayload {
                text: "Test 3".to_string(),
            }),
            metadata: None,
        },
        AgentEvent {
            id: tool3_id,
            session_id,
            parent_id: Some(user3_id),
            timestamp: base_time,
            stream_id: StreamId::Main,
            payload: EventPayload::ToolCall(ToolCallPayload::from_raw(
                "read".to_string(),
                serde_json::json!({"file": "test.txt"}),
                Some("call_3".to_string()),
            )),
            metadata: None,
        },
        AgentEvent {
            id: result3_id,
            session_id,
            parent_id: Some(tool3_id),
            timestamp: base_time,
            stream_id: StreamId::Main,
            payload: EventPayload::ToolResult(ToolResultPayload {
                output: "file content".to_string(),
                tool_call_id: tool3_id,
                is_error: false,
            }),
            metadata: None,
        },
        // Turn 4: Reasoning only -> InProgress
        AgentEvent {
            id: user4_id,
            session_id,
            parent_id: Some(result3_id),
            timestamp: base_time,
            stream_id: StreamId::Main,
            payload: EventPayload::User(UserPayload {
                text: "Test 4".to_string(),
            }),
            metadata: None,
        },
        AgentEvent {
            id: reasoning4_id,
            session_id,
            parent_id: Some(user4_id),
            timestamp: base_time,
            stream_id: StreamId::Main,
            payload: EventPayload::Reasoning(ReasoningPayload {
                text: "Thinking...".to_string(),
            }),
            metadata: None,
        },
        // Turn 5: Tool with error -> Failed
        AgentEvent {
            id: user5_id,
            session_id,
            parent_id: Some(reasoning4_id),
            timestamp: base_time,
            stream_id: StreamId::Main,
            payload: EventPayload::User(UserPayload {
                text: "Test 5".to_string(),
            }),
            metadata: None,
        },
        AgentEvent {
            id: tool5_id,
            session_id,
            parent_id: Some(user5_id),
            timestamp: base_time,
            stream_id: StreamId::Main,
            payload: EventPayload::ToolCall(ToolCallPayload::from_raw(
                "bash".to_string(),
                serde_json::json!({"command": "invalid"}),
                Some("call_5".to_string()),
            )),
            metadata: None,
        },
        AgentEvent {
            id: result5_id,
            session_id,
            parent_id: Some(tool5_id),
            timestamp: base_time,
            stream_id: StreamId::Main,
            payload: EventPayload::ToolResult(ToolResultPayload {
                output: "command not found".to_string(),
                tool_call_id: tool5_id,
                is_error: true,
            }),
            metadata: None,
        },
    ];

    let session = assemble_session(&events).expect("Failed to assemble session");

    assert_eq!(session.turns.len(), 5, "Should have 5 turns");

    // Verify status for each turn
    assert_eq!(
        session.turns[0].steps[0].status,
        StepStatus::Done,
        "Turn 1: Message should be Done"
    );

    assert_eq!(
        session.turns[1].steps[0].status,
        StepStatus::InProgress,
        "Turn 2: Tool without result should be InProgress"
    );

    assert_eq!(
        session.turns[2].steps[0].status,
        StepStatus::Done,
        "Turn 3: Tool with result should be Done"
    );

    assert_eq!(
        session.turns[3].steps[0].status,
        StepStatus::InProgress,
        "Turn 4: Reasoning only should be InProgress"
    );

    assert_eq!(
        session.turns[4].steps[0].status,
        StepStatus::Failed,
        "Turn 5: Tool error should be Failed"
    );
}
