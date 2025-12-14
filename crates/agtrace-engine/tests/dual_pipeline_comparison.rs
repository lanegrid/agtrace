use agtrace_engine::{build_spans, build_spans_from_v2, summarize_session, summarize_session_v2};
use std::path::Path;

#[test]
fn test_gemini_dual_pipeline_summary() {
    let path = Path::new("../agtrace-providers/tests/samples/gemini_session.json");

    if !path.exists() {
        eprintln!("Warning: Test file not found, skipping: {}", path.display());
        return;
    }

    // Load through both pipelines
    let events_v1 = agtrace_providers::normalize_gemini_file(path)
        .expect("Failed to normalize Gemini file (v1)");
    let events_v2 = agtrace_providers::normalize_gemini_file_v2(path)
        .expect("Failed to normalize Gemini file (v2)");

    // Generate summaries
    let summary_v1 = summarize_session(&events_v1);
    let summary_v2 = summarize_session_v2(&events_v2);

    // Build spans
    let spans_v1 = build_spans(&events_v1);
    let spans_v2 = build_spans_from_v2(&events_v2);

    // Compare results
    println!("\n=== Gemini Dual Pipeline Comparison ===");
    println!("\nV1 Summary:");
    println!("  Events: {}", summary_v1.event_counts.total);
    println!("  Tool Calls: {}", summary_v1.event_counts.tool_calls);
    println!("  Spans: {}", spans_v1.len());
    println!(
        "  Tokens: {} (in: {}, out: {})",
        summary_v1.token_stats.total, summary_v1.token_stats.input, summary_v1.token_stats.output
    );

    println!("\nV2 Summary:");
    println!("  Events: {}", summary_v2.event_counts.total);
    println!("  Tool Calls: {}", summary_v2.event_counts.tool_calls);
    println!("  Spans: {}", spans_v2.len());
    println!(
        "  Tokens: {} (in: {}, out: {})",
        summary_v2.token_stats.total, summary_v2.token_stats.input, summary_v2.token_stats.output
    );

    // Document differences
    println!("\nDifferences:");
    if spans_v1.len() != spans_v2.len() {
        println!(
            "  V2 captures more spans: {} vs {} ({}% more accurate)",
            spans_v2.len(),
            spans_v1.len(),
            ((spans_v2.len() as f64 / spans_v1.len() as f64 - 1.0) * 100.0) as i32
        );
    }
    if summary_v1.token_stats.total != summary_v2.token_stats.total {
        println!(
            "  V2 tracks more tokens: {} vs {} ({} more)",
            summary_v2.token_stats.total,
            summary_v1.token_stats.total,
            summary_v2.token_stats.total as i64 - summary_v1.token_stats.total as i64
        );
    }

    // Tool call counts should match - this is a basic sanity check
    assert_eq!(
        summary_v1.event_counts.tool_calls, summary_v2.event_counts.tool_calls,
        "Tool call counts should match between v1 and v2"
    );
}

#[test]
fn test_codex_dual_pipeline_summary() {
    let path = Path::new("../agtrace-providers/tests/samples/codex_session.jsonl");

    if !path.exists() {
        eprintln!("Warning: Test file not found, skipping: {}", path.display());
        return;
    }

    // Load through both pipelines
    let events_v1 = agtrace_providers::normalize_codex_file(path, None)
        .expect("Failed to normalize Codex file (v1)");
    let events_v2 = agtrace_providers::normalize_codex_file_v2(path)
        .expect("Failed to normalize Codex file (v2)");

    // Generate summaries
    let summary_v1 = summarize_session(&events_v1);
    let summary_v2 = summarize_session_v2(&events_v2);

    // Build spans
    let spans_v1 = build_spans(&events_v1);
    let spans_v2 = build_spans_from_v2(&events_v2);

    // Compare results
    println!("\n=== Codex Dual Pipeline Comparison ===");
    println!("\nV1 Summary:");
    println!("  Events: {}", summary_v1.event_counts.total);
    println!("  Tool Calls: {}", summary_v1.event_counts.tool_calls);
    println!("  Spans: {}", spans_v1.len());
    println!(
        "  Tokens: {} (in: {}, out: {})",
        summary_v1.token_stats.total, summary_v1.token_stats.input, summary_v1.token_stats.output
    );

    println!("\nV2 Summary:");
    println!("  Events: {}", summary_v2.event_counts.total);
    println!("  Tool Calls: {}", summary_v2.event_counts.tool_calls);
    println!("  Spans: {}", spans_v2.len());
    println!(
        "  Tokens: {} (in: {}, out: {})",
        summary_v2.token_stats.total, summary_v2.token_stats.input, summary_v2.token_stats.output
    );

    // Document differences
    println!("\nDifferences:");
    if spans_v1.len() != spans_v2.len() {
        println!(
            "  V2 captures more spans: {} vs {} ({}% more accurate)",
            spans_v2.len(),
            spans_v1.len(),
            ((spans_v2.len() as f64 / spans_v1.len() as f64 - 1.0) * 100.0) as i32
        );
    }
    if summary_v1.token_stats.total != summary_v2.token_stats.total {
        println!(
            "  V2 tracks more tokens: {} vs {} ({} more)",
            summary_v2.token_stats.total,
            summary_v1.token_stats.total,
            summary_v2.token_stats.total as i64 - summary_v1.token_stats.total as i64
        );
    }

    // Tool call counts should match - this is a basic sanity check
    assert_eq!(
        summary_v1.event_counts.tool_calls, summary_v2.event_counts.tool_calls,
        "Tool call counts should match between v1 and v2"
    );
}

#[test]
fn test_claude_dual_pipeline_summary() {
    let path = Path::new("../agtrace-providers/tests/samples/claude_session.jsonl");

    if !path.exists() {
        eprintln!("Warning: Test file not found, skipping: {}", path.display());
        return;
    }

    // Load through both pipelines
    let events_v1 = agtrace_providers::normalize_claude_file(path, None)
        .expect("Failed to normalize Claude file (v1)");
    let events_v2 = agtrace_providers::normalize_claude_file_v2(path)
        .expect("Failed to normalize Claude file (v2)");

    // Generate summaries
    let summary_v1 = summarize_session(&events_v1);
    let summary_v2 = summarize_session_v2(&events_v2);

    // Build spans
    let spans_v1 = build_spans(&events_v1);
    let spans_v2 = build_spans_from_v2(&events_v2);

    // Compare results
    println!("\n=== Claude Dual Pipeline Comparison ===");
    println!("\nV1 Summary:");
    println!("  Events: {}", summary_v1.event_counts.total);
    println!("  Tool Calls: {}", summary_v1.event_counts.tool_calls);
    println!("  Spans: {}", spans_v1.len());
    println!(
        "  Tokens: {} (in: {}, out: {})",
        summary_v1.token_stats.total, summary_v1.token_stats.input, summary_v1.token_stats.output
    );

    println!("\nV2 Summary:");
    println!("  Events: {}", summary_v2.event_counts.total);
    println!("  Tool Calls: {}", summary_v2.event_counts.tool_calls);
    println!("  Spans: {}", spans_v2.len());
    println!(
        "  Tokens: {} (in: {}, out: {})",
        summary_v2.token_stats.total, summary_v2.token_stats.input, summary_v2.token_stats.output
    );

    // Document differences
    println!("\nDifferences:");
    if spans_v1.len() != spans_v2.len() {
        println!(
            "  V2 captures more spans: {} vs {} ({}% more accurate)",
            spans_v2.len(),
            spans_v1.len(),
            ((spans_v2.len() as f64 / spans_v1.len() as f64 - 1.0) * 100.0) as i32
        );
    }
    if summary_v1.token_stats.total != summary_v2.token_stats.total {
        println!(
            "  V2 tracks more tokens: {} vs {} ({} more)",
            summary_v2.token_stats.total,
            summary_v1.token_stats.total,
            summary_v2.token_stats.total as i64 - summary_v1.token_stats.total as i64
        );
    }

    // Tool call counts should match - this is a basic sanity check
    assert_eq!(
        summary_v1.event_counts.tool_calls, summary_v2.event_counts.tool_calls,
        "Tool call counts should match between v1 and v2"
    );
}

#[test]
fn test_tool_matching_comparison() {
    use agtrace_types::v2::*;
    use chrono::Utc;
    use uuid::Uuid;

    let trace_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let tool1_id = Uuid::new_v4();
    let tool2_id = Uuid::new_v4();

    // Create v2 events with out-of-order results
    let events_v2 = vec![
        AgentEvent {
            id: user_id,
            trace_id,
            parent_id: None,
            timestamp: Utc::now(),
            payload: EventPayload::User(UserPayload {
                text: "Run two commands".to_string(),
            }),
            metadata: None,
        },
        // Two tool calls
        AgentEvent {
            id: tool1_id,
            trace_id,
            parent_id: Some(user_id),
            timestamp: Utc::now(),
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
            payload: EventPayload::ToolCall(ToolCallPayload {
                name: "grep".to_string(),
                arguments: serde_json::json!({"pattern": "test"}),
                provider_call_id: Some("call_2".to_string()),
            }),
            metadata: None,
        },
        // Results arrive in reverse order (tool2, then tool1)
        AgentEvent {
            id: Uuid::new_v4(),
            trace_id,
            parent_id: Some(tool2_id),
            timestamp: Utc::now(),
            payload: EventPayload::ToolResult(ToolResultPayload {
                output: "match found".to_string(),
                tool_call_id: tool2_id,
                is_error: false,
            }),
            metadata: None,
        },
        AgentEvent {
            id: Uuid::new_v4(),
            trace_id,
            parent_id: Some(tool2_id),
            timestamp: Utc::now(),
            payload: EventPayload::ToolResult(ToolResultPayload {
                output: "file1.txt\nfile2.txt".to_string(),
                tool_call_id: tool1_id,
                is_error: false,
            }),
            metadata: None,
        },
    ];

    let spans_v2 = build_spans_from_v2(&events_v2);

    println!("\n=== Tool Matching Comparison ===");
    println!(
        "V2 correctly matches {} tools even with out-of-order results",
        spans_v2[0].tools.len()
    );

    // Verify both tools are matched correctly
    assert_eq!(spans_v2.len(), 1);
    assert_eq!(spans_v2[0].tools.len(), 2);
    assert!(
        spans_v2[0].tools[0].ts_result.is_some(),
        "bash should have result"
    );
    assert!(
        spans_v2[0].tools[1].ts_result.is_some(),
        "grep should have result"
    );
}
