#![allow(clippy::format_in_format_args)] // Intentional for colored terminal output

use crate::output::{format_spans_compact, print_events_timeline, CompactFormatOpts};
use crate::session_loader::{LoadOptions, SessionLoader};
use agtrace_engine::build_spans_from_events;
use agtrace_index::Database;
use agtrace_types::v2::{AgentEvent, EventPayload};
use agtrace_types::{AgentEventV1, EventType};
use anyhow::{Context, Result};
use is_terminal::IsTerminal;
use std::fs;
use std::io;

#[allow(clippy::too_many_arguments)]
pub fn handle(
    db: &Database,
    session_id: String,
    raw: bool,
    json: bool,
    _timeline: bool,
    hide: Option<Vec<String>>,
    only: Option<Vec<String>>,
    _full: bool, // Kept for backwards compatibility, but now default
    short: bool,
    style: String,
) -> Result<()> {
    // Detect if output is being piped (not a terminal)
    let is_tty = io::stdout().is_terminal();
    let enable_color = is_tty;

    // Handle raw mode (display raw files without normalization)
    if raw {
        let log_files = db.get_session_files(&session_id)?;
        let main_files: Vec<_> = log_files
            .into_iter()
            .filter(|f| f.role != "sidechain")
            .collect();

        for log_file in &main_files {
            let content = fs::read_to_string(&log_file.path)
                .with_context(|| format!("Failed to read file: {}", log_file.path))?;
            println!("{}", content);
        }
        return Ok(());
    }

    // Load and normalize events (using v2 pipeline)
    let loader = SessionLoader::new(db);
    let options = LoadOptions::default();
    let all_events_v2 = loader.load_events_v2(&session_id, &options)?;

    // Filter events based on --hide and --only options
    let filtered_events = filter_events_v2(&all_events_v2, hide.as_ref(), only.as_ref());

    if json {
        println!("{}", serde_json::to_string_pretty(&filtered_events)?);
    } else if style == "compact" {
        let spans = build_spans_from_events(&filtered_events);
        let opts = CompactFormatOpts {
            enable_color,
            relative_time: true,
        };
        let lines = format_spans_compact(&spans, &opts);
        for line in lines {
            println!("{}", line);
        }
    } else {
        // For timeline view, convert v2 events to v1 format for now
        // TODO: Update print_events_timeline to work with v2 events
        let v1_events = convert_v2_to_v1(&filtered_events);
        let truncate = short;
        print_events_timeline(&v1_events, truncate, enable_color);
    }

    Ok(())
}

/// Filter v2 events based on hide/only patterns
fn filter_events_v2(
    events: &[AgentEvent],
    hide: Option<&Vec<String>>,
    only: Option<&Vec<String>>,
) -> Vec<AgentEvent> {
    let mut filtered = events.to_vec();

    // Apply --only filter (whitelist)
    if let Some(only_patterns) = only {
        filtered.retain(|e| {
            only_patterns.iter().any(|pattern| {
                let pattern_lower = pattern.to_lowercase();
                match &e.payload {
                    EventPayload::User(_) => pattern_lower == "user",
                    EventPayload::Message(_) => {
                        pattern_lower == "assistant" || pattern_lower == "message"
                    }
                    EventPayload::ToolCall(_) | EventPayload::ToolResult(_) => {
                        pattern_lower == "tool"
                    }
                    EventPayload::Reasoning(_) => pattern_lower == "reasoning",
                    EventPayload::TokenUsage(_) => {
                        pattern_lower == "token" || pattern_lower == "tokenusage"
                    }
                }
            })
        });
    }

    // Apply --hide filter (blacklist)
    if let Some(hide_patterns) = hide {
        filtered.retain(|e| {
            !hide_patterns.iter().any(|pattern| {
                let pattern_lower = pattern.to_lowercase();
                match &e.payload {
                    EventPayload::User(_) => pattern_lower == "user",
                    EventPayload::Message(_) => {
                        pattern_lower == "assistant" || pattern_lower == "message"
                    }
                    EventPayload::ToolCall(_) | EventPayload::ToolResult(_) => {
                        pattern_lower == "tool"
                    }
                    EventPayload::Reasoning(_) => pattern_lower == "reasoning",
                    EventPayload::TokenUsage(_) => {
                        pattern_lower == "token" || pattern_lower == "tokenusage"
                    }
                }
            })
        });
    }

    filtered
}

/// Convert v2 events to v1 format for compatibility with legacy output functions
fn convert_v2_to_v1(events: &[AgentEvent]) -> Vec<AgentEventV1> {
    use agtrace_types::Source;

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
