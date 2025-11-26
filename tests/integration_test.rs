use agtrace::model::*;
use agtrace::parser::codex;
use chrono::Utc;
use std::path::PathBuf;

#[test]
fn test_execution_creation() {
    let execution = Execution {
        id: "test-1".to_string(),
        agent: Agent::ClaudeCode {
            model: "claude-sonnet-4".to_string(),
            version: "2.0.0".to_string(),
        },
        project_path: PathBuf::from("/test/project"),
        git_branch: Some("main".to_string()),
        started_at: Utc::now(),
        ended_at: Some(Utc::now()),
        summaries: vec!["Test summary".to_string()],
        events: vec![],
        metrics: ExecutionMetrics::default(),
    };

    assert_eq!(execution.id, "test-1");
    assert_eq!(execution.summaries.len(), 1);
}

#[test]
fn test_compute_metrics() {
    let now = Utc::now();
    let mut execution = Execution {
        id: "test-2".to_string(),
        agent: Agent::Codex {
            model: "gpt-5-codex".to_string(),
        },
        project_path: PathBuf::from("/test/project"),
        git_branch: None,
        started_at: now,
        ended_at: Some(now + chrono::Duration::seconds(60)),
        summaries: vec![],
        events: vec![
            Event::UserMessage {
                content: "Hello".to_string(),
                timestamp: now,
            },
            Event::AssistantMessage {
                content: "Hi".to_string(),
                timestamp: now,
            },
            Event::ToolCall {
                name: "Read".to_string(),
                input: serde_json::json!({"file_path": "/test/file.rs"}),
                call_id: Some("call-1".to_string()),
                timestamp: now,
            },
            Event::ToolResult {
                call_id: Some("call-1".to_string()),
                output: "file contents".to_string(),
                exit_code: None,
                duration_ms: Some(100),
                timestamp: now,
            },
        ],
        metrics: ExecutionMetrics::default(),
    };

    execution.compute_metrics();

    assert_eq!(execution.metrics.user_message_count, 1);
    assert_eq!(execution.metrics.assistant_message_count, 1);
    assert_eq!(execution.metrics.tool_call_count, 1);
    assert_eq!(execution.metrics.duration_seconds, Some(60));
    assert_eq!(execution.metrics.files_read.len(), 1);
}

#[test]
fn test_serialization() {
    let execution = Execution {
        id: "test-3".to_string(),
        agent: Agent::ClaudeCode {
            model: "claude-sonnet-4".to_string(),
            version: "2.0.0".to_string(),
        },
        project_path: PathBuf::from("/test/project"),
        git_branch: Some("main".to_string()),
        started_at: Utc::now(),
        ended_at: None,
        summaries: vec![],
        events: vec![],
        metrics: ExecutionMetrics::default(),
    };

    // Test JSON serialization
    let json = serde_json::to_string(&execution).unwrap();
    let deserialized: Execution = serde_json::from_str(&json).unwrap();

    assert_eq!(execution.id, deserialized.id);
    assert_eq!(execution.git_branch, deserialized.git_branch);
}

#[test]
fn test_event_types() {
    let now = Utc::now();

    let user_msg = Event::UserMessage {
        content: "test".to_string(),
        timestamp: now,
    };

    let json = serde_json::to_string(&user_msg).unwrap();
    assert!(json.contains("user_message"));

    let thinking = Event::Thinking {
        content: "reasoning".to_string(),
        duration_ms: Some(1000),
        timestamp: now,
    };

    let json = serde_json::to_string(&thinking).unwrap();
    assert!(json.contains("thinking"));
}

#[test]
fn test_metrics_aggregation() {
    let mut metrics = ExecutionMetrics::default();
    metrics.input_tokens = 100;
    metrics.output_tokens = 50;
    metrics.tool_calls_by_name.insert("Read".to_string(), 5);
    metrics.tool_calls_by_name.insert("Write".to_string(), 2);

    assert_eq!(metrics.input_tokens, 100);
    assert_eq!(metrics.output_tokens, 50);
    assert_eq!(metrics.tool_calls_by_name.len(), 2);
}

#[test]
fn test_codex_parsing() {
    // Parse the codex fixture
    let fixture_dir = PathBuf::from("tests/fixtures/codex");
    let executions = codex::parse_dir(&fixture_dir).expect("Failed to parse codex fixtures");

    // Find the basic fixture
    let basic_exec = executions
        .iter()
        .find(|e| e.id.contains("019a1234-5678-7890-abcd-ef1234567890"))
        .expect("Basic fixture not found");

    // Verify session metadata
    assert_eq!(basic_exec.id, "codex-019a1234-5678-7890-abcd-ef1234567890");
    assert_eq!(
        basic_exec.project_path,
        PathBuf::from("/Users/testuser/projects/test-project")
    );
    assert_eq!(basic_exec.git_branch, Some("main".to_string()));

    // Verify agent model is extracted
    match &basic_exec.agent {
        Agent::Codex { model } => {
            assert_eq!(model, "gpt-5-codex");
        }
        _ => panic!("Expected Codex agent"),
    }

    // Verify events are parsed
    assert!(!basic_exec.events.is_empty(), "Should have events");

    // Check that we have user messages
    let has_user_msg = basic_exec
        .events
        .iter()
        .any(|e| matches!(e, Event::UserMessage { .. }));
    assert!(has_user_msg, "Should have user message");

    // Check that we have assistant messages
    let has_assistant_msg = basic_exec
        .events
        .iter()
        .any(|e| matches!(e, Event::AssistantMessage { .. }));
    assert!(has_assistant_msg, "Should have assistant message");

    // Check that we have tool calls
    let has_tool_call = basic_exec
        .events
        .iter()
        .any(|e| matches!(e, Event::ToolCall { .. }));
    assert!(has_tool_call, "Should have tool call");

    // Check that we have tool results
    let has_tool_result = basic_exec
        .events
        .iter()
        .any(|e| matches!(e, Event::ToolResult { .. }));
    assert!(has_tool_result, "Should have tool result");
}

#[test]
fn test_codex_edge_cases() {
    // Parse the codex edge cases fixture
    let fixture_dir = PathBuf::from("tests/fixtures/codex");
    let executions = codex::parse_dir(&fixture_dir).expect("Failed to parse codex fixtures");

    // Find the edge cases fixture
    let edge_case_exec = executions
        .iter()
        .find(|e| e.id.contains("019a9999-8888-7777-6666-555544443333"))
        .expect("Edge cases fixture not found");

    // Verify it handles empty instructions
    assert!(
        edge_case_exec.summaries.is_empty(),
        "Should have empty summaries for empty instructions"
    );

    // Verify it parses failed function calls
    let tool_results: Vec<_> = edge_case_exec
        .events
        .iter()
        .filter_map(|e| match e {
            Event::ToolResult { exit_code, .. } => Some(exit_code),
            _ => None,
        })
        .collect();

    assert!(!tool_results.is_empty(), "Should have tool results");

    // Check for non-zero exit code (failed command)
    let has_failure = tool_results
        .iter()
        .any(|ec| ec.is_some() && ec.unwrap() != 0);
    assert!(
        has_failure,
        "Should have failed tool call with non-zero exit code"
    );
}
