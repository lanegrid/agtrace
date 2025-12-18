use agtrace_engine::build_spans_from_events;
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
fn test_gemini_span_building() {
    let events = load_events_from_fixture("gemini_events.json");

    let spans = build_spans_from_events(&events);

    // Verify spans were created
    assert!(!spans.is_empty(), "Expected at least one span");

    // Snapshot the spans
    insta::assert_json_snapshot!("gemini_spans", spans);
}

#[test]
fn test_codex_span_building() {
    let events = load_events_from_fixture("codex_events.json");

    let spans = build_spans_from_events(&events);

    // Verify spans were created
    assert!(!spans.is_empty(), "Expected at least one span");

    // Snapshot the spans
    insta::assert_json_snapshot!("codex_spans", spans);
}

#[test]
fn test_claude_span_building() {
    let events = load_events_from_fixture("claude_events.json");

    let spans = build_spans_from_events(&events);

    // Verify spans were created
    assert!(!spans.is_empty(), "Expected at least one span");

    // Snapshot the spans
    insta::assert_json_snapshot!("claude_spans", spans);
}

#[test]
fn test_tool_matching_accuracy() {
    use agtrace_types::*;
    use chrono::Utc;
    use uuid::Uuid;

    let trace_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let tool1_id = Uuid::new_v4();
    let tool2_id = Uuid::new_v4();
    let tool3_id = Uuid::new_v4();

    // Create events with out-of-order results
    let events = vec![
        AgentEvent {
            id: user_id,
            trace_id,
            parent_id: None,
            timestamp: Utc::now(),
            stream_id: agtrace_types::StreamId::Main,
            payload: EventPayload::User(UserPayload {
                text: "Run three commands".to_string(),
            }),
            metadata: None,
        },
        // Three parallel tool calls
        AgentEvent {
            id: tool1_id,
            trace_id,
            parent_id: Some(user_id),
            timestamp: Utc::now(),
            stream_id: agtrace_types::StreamId::Main,
            payload: EventPayload::ToolCall(ToolCallPayload {
                name: "bash".to_string(),
                arguments: serde_json::json!({"command": "ls"}),
                provider_call_id: Some("call_1".to_string()),
            }),
            metadata: None,
        },
        AgentEvent {
            id: tool2_id,
            trace_id,
            parent_id: Some(tool1_id),
            timestamp: Utc::now(),
            stream_id: agtrace_types::StreamId::Main,
            payload: EventPayload::ToolCall(ToolCallPayload {
                name: "grep".to_string(),
                arguments: serde_json::json!({"pattern": "test"}),
                provider_call_id: Some("call_2".to_string()),
            }),
            metadata: None,
        },
        AgentEvent {
            id: tool3_id,
            trace_id,
            parent_id: Some(tool2_id),
            timestamp: Utc::now(),
            stream_id: agtrace_types::StreamId::Main,
            payload: EventPayload::ToolCall(ToolCallPayload {
                name: "python".to_string(),
                arguments: serde_json::json!({"command": "print('hello')"}),
                provider_call_id: Some("call_3".to_string()),
            }),
            metadata: None,
        },
        // Results arrive in reverse order: tool3, tool1, tool2
        AgentEvent {
            id: Uuid::new_v4(),
            trace_id,
            parent_id: Some(tool3_id),
            timestamp: Utc::now(),
            stream_id: agtrace_types::StreamId::Main,
            payload: EventPayload::ToolResult(ToolResultPayload {
                output: "hello".to_string(),
                tool_call_id: tool3_id,
                is_error: false,
            }),
            metadata: None,
        },
        AgentEvent {
            id: Uuid::new_v4(),
            trace_id,
            parent_id: Some(tool3_id),
            timestamp: Utc::now(),
            stream_id: agtrace_types::StreamId::Main,
            payload: EventPayload::ToolResult(ToolResultPayload {
                output: "file1.txt\nfile2.txt".to_string(),
                tool_call_id: tool1_id,
                is_error: false,
            }),
            metadata: None,
        },
        AgentEvent {
            id: Uuid::new_v4(),
            trace_id,
            parent_id: Some(tool3_id),
            timestamp: Utc::now(),
            stream_id: agtrace_types::StreamId::Main,
            payload: EventPayload::ToolResult(ToolResultPayload {
                output: "match found".to_string(),
                tool_call_id: tool2_id,
                is_error: false,
            }),
            metadata: None,
        },
    ];

    let spans = build_spans_from_events(&events);

    assert_eq!(spans.len(), 1);
    let span = &spans[0];

    // All three tools should be matched correctly
    assert_eq!(span.tools.len(), 3);
    assert_eq!(span.tools[0].tool_name, "bash");
    assert!(span.tools[0].ts_result.is_some(), "bash should have result");

    assert_eq!(span.tools[1].tool_name, "grep");
    assert!(span.tools[1].ts_result.is_some(), "grep should have result");

    assert_eq!(span.tools[2].tool_name, "python");
    assert!(
        span.tools[2].ts_result.is_some(),
        "python should have result"
    );
}
