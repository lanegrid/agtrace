use agtrace_engine::assemble_session_from_events;
use std::path::Path;

// Helper function to redact UUIDs from JSON for snapshot testing
fn redact_uuids(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, val) in map.iter_mut() {
                if key == "id"
                    || key == "session_id"
                    || key == "trace_id"
                    || key == "parent_id"
                    || key == "tool_call_id"
                    || key == "event_id"
                {
                    if val.is_string() || val.is_null() {
                        *val = serde_json::Value::String("<UUID_REDACTED>".to_string());
                    }
                } else {
                    redact_uuids(val);
                }
            }
        }
        serde_json::Value::Array(arr) => {
            for val in arr.iter_mut() {
                redact_uuids(val);
            }
        }
        _ => {}
    }
}

#[test]
fn test_gemini_session_assembly() {
    let path = Path::new("../agtrace-providers/tests/samples/gemini_session.json");

    if !path.exists() {
        eprintln!("Warning: Test file not found, skipping: {}", path.display());
        return;
    }

    let events =
        agtrace_providers::normalize_gemini_file_v2(path).expect("Failed to normalize Gemini file");

    let session = assemble_session_from_events(&events).expect("Failed to assemble session");

    assert!(!session.turns.is_empty(), "Expected at least one turn");

    let mut value = serde_json::to_value(&session).unwrap();
    redact_uuids(&mut value);
    insta::assert_json_snapshot!("gemini_session_assembly", value);
}

#[test]
fn test_codex_session_assembly() {
    let path = Path::new("../agtrace-providers/tests/samples/codex_session.jsonl");

    if !path.exists() {
        eprintln!("Warning: Test file not found, skipping: {}", path.display());
        return;
    }

    let events =
        agtrace_providers::normalize_codex_file_v2(path).expect("Failed to normalize Codex file");

    let session = assemble_session_from_events(&events).expect("Failed to assemble session");

    assert!(!session.turns.is_empty(), "Expected at least one turn");

    let mut value = serde_json::to_value(&session).unwrap();
    redact_uuids(&mut value);
    insta::assert_json_snapshot!("codex_session_assembly", value);
}

#[test]
fn test_claude_session_assembly() {
    let path = Path::new("../agtrace-providers/tests/samples/claude_session.jsonl");

    if !path.exists() {
        eprintln!("Warning: Test file not found, skipping: {}", path.display());
        return;
    }

    let events =
        agtrace_providers::normalize_claude_file_v2(path).expect("Failed to normalize Claude file");

    let session = assemble_session_from_events(&events).expect("Failed to assemble session");

    assert!(!session.turns.is_empty(), "Expected at least one turn");

    let mut value = serde_json::to_value(&session).unwrap();
    redact_uuids(&mut value);
    insta::assert_json_snapshot!("claude_session_assembly", value);
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

    let mut value = serde_json::to_value(&session).unwrap();
    redact_uuids(&mut value);
    insta::assert_json_snapshot!("session_assembly_structure", value);
}
