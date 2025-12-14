use crate::session_loader::{LoadOptions, SessionLoader};
use agtrace_engine::analysis::{self, AnalysisReport, Detector};
use agtrace_index::Database;
use agtrace_types::v2::{AgentEvent, EventPayload};
use agtrace_types::{AgentEventV1, EventType, Source};
use anyhow::Result;
use owo_colors::OwoColorize;

pub fn handle(db: &Database, session_id: String, detect: String, format: String) -> Result<()> {
    let loader = SessionLoader::new(db);
    let options = LoadOptions::default();
    // Load using v2 pipeline, then convert to v1 for analysis
    // TODO: Create native v2 analysis functions
    let events_v2 = loader.load_events_v2(&session_id, &options)?;
    let all_events = convert_v2_to_v1(&events_v2);

    let detectors: Vec<Detector> = if detect == "all" {
        Detector::all()
    } else {
        detect
            .split(',')
            .filter_map(|d| match d.trim().parse() {
                Ok(detector) => Some(detector),
                Err(e) => {
                    eprintln!("Warning: {}", e);
                    None
                }
            })
            .collect()
    };

    let report = analysis::analyze(session_id.clone(), &all_events, detectors);

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_report(&report);
    }

    Ok(())
}

fn print_report(report: &AnalysisReport) {
    println!(
        "Analysis Report for Session: {}",
        report.session_id.bright_blue()
    );

    let score_colored = if report.score >= 90 {
        format!("{}", report.score.to_string().green())
    } else if report.score >= 70 {
        format!("{}", report.score.to_string().yellow())
    } else {
        format!("{}", report.score.to_string().red())
    };

    let warning_text = if report.warnings.len() == 1 {
        "1 Warning"
    } else {
        &format!("{} Warnings", report.warnings.len())
    };

    println!("Score: {}/100 ({})", score_colored, warning_text);
    println!();

    for warning in &report.warnings {
        println!(
            "{} {} (Count: {})",
            "[WARN]".yellow(),
            warning.pattern.bold(),
            warning.count
        );
        println!("  Span: {}", warning.span);
        println!("  Insight: {}", warning.insight);
        println!();
    }

    for info in &report.info {
        println!("{} {}", "[INFO]".cyan(), info.category.bold());
        for detail in &info.details {
            println!("  - {}", detail);
        }
        println!();
    }
}

/// Convert v2 events to v1 format for compatibility with legacy analysis functions
fn convert_v2_to_v1(events: &[AgentEvent]) -> Vec<AgentEventV1> {
    events
        .iter()
        .filter_map(|e| {
            // Skip TokenUsage events in v1 representation
            if matches!(e.payload, EventPayload::TokenUsage(_)) {
                return None;
            }

            let event_type = match &e.payload {
                EventPayload::User(_) => EventType::UserMessage,
                EventPayload::Message(_) => EventType::AssistantMessage,
                EventPayload::ToolCall(_) => EventType::ToolCall,
                EventPayload::ToolResult(_) => EventType::ToolResult,
                EventPayload::Reasoning(_) => EventType::Reasoning,
                EventPayload::TokenUsage(_) => return None,
            };

            let text = match &e.payload {
                EventPayload::User(p) => Some(p.text.clone()),
                EventPayload::Message(p) => Some(p.text.clone()),
                EventPayload::Reasoning(p) => Some(p.text.clone()),
                EventPayload::ToolResult(p) => Some(p.output.clone()),
                EventPayload::ToolCall(p) => Some(format!("{}: {}", p.name, p.arguments)),
                _ => None,
            };

            let tool_name = match &e.payload {
                EventPayload::ToolCall(p) => Some(p.name.clone()),
                _ => None,
            };

            Some(AgentEventV1 {
                schema_version: AgentEventV1::SCHEMA_VERSION.to_string(),
                source: Source::new("unknown"),
                project_hash: String::new(),
                project_root: None,
                session_id: Some(e.trace_id.to_string()),
                event_id: Some(e.id.to_string()),
                parent_event_id: e.parent_id.map(|id| id.to_string()),
                ts: e.timestamp.to_rfc3339(),
                event_type,
                role: None,
                channel: None,
                text,
                context: None,
                policy: None,
                tool_name,
                tool_call_id: None,
                tool_status: None,
                tool_latency_ms: None,
                tool_exit_code: None,
                file_path: None,
                file_language: None,
                file_op: None,
                model: None,
                tokens_input: None,
                tokens_output: None,
                tokens_total: None,
                tokens_cached: None,
                tokens_thinking: None,
                tokens_tool: None,
                agent_id: None,
                raw: serde_json::Value::Null,
            })
        })
        .collect()
}
