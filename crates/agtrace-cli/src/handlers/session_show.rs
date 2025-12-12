#![allow(clippy::format_in_format_args)] // Intentional for colored terminal output

use crate::output::{print_events_timeline, print_turns_compact};
use crate::session_loader::{LoadOptions, SessionLoader};
use agtrace_engine::build_turns;
use agtrace_index::Database;
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

    // Load and normalize events
    let loader = SessionLoader::new(db);
    let options = LoadOptions::default();
    let all_events = loader.load_events(&session_id, &options)?;

    // Filter events based on --hide and --only options
    let filtered_events = filter_events(&all_events, hide.as_ref(), only.as_ref());

    if json {
        println!("{}", serde_json::to_string_pretty(&filtered_events)?);
    } else if style == "compact" {
        let turns = build_turns(&filtered_events);
        print_turns_compact(&turns, enable_color);
    } else {
        // Default is full display, --short enables truncation
        let truncate = short;
        print_events_timeline(&filtered_events, truncate, enable_color);
    }

    Ok(())
}

/// Filter events based on hide/only patterns
fn filter_events(
    events: &[AgentEventV1],
    hide: Option<&Vec<String>>,
    only: Option<&Vec<String>>,
) -> Vec<AgentEventV1> {
    let mut filtered = events.to_vec();

    // Apply --only filter (whitelist)
    if let Some(only_patterns) = only {
        filtered.retain(|e| {
            let event_type = format!("{:?}", e.event_type).to_lowercase();
            only_patterns.iter().any(|pattern| {
                let pattern_lower = pattern.to_lowercase();
                event_type.contains(&pattern_lower)
                    || pattern_lower == "user" && matches!(e.event_type, EventType::UserMessage)
                    || pattern_lower == "assistant"
                        && matches!(e.event_type, EventType::AssistantMessage)
                    || pattern_lower == "tool"
                        && (matches!(e.event_type, EventType::ToolCall)
                            || matches!(e.event_type, EventType::ToolResult))
                    || pattern_lower == "reasoning" && matches!(e.event_type, EventType::Reasoning)
            })
        });
    }

    // Apply --hide filter (blacklist)
    if let Some(hide_patterns) = hide {
        filtered.retain(|e| {
            let event_type = format!("{:?}", e.event_type).to_lowercase();
            !hide_patterns.iter().any(|pattern| {
                let pattern_lower = pattern.to_lowercase();
                event_type.contains(&pattern_lower)
                    || pattern_lower == "user" && matches!(e.event_type, EventType::UserMessage)
                    || pattern_lower == "assistant"
                        && matches!(e.event_type, EventType::AssistantMessage)
                    || pattern_lower == "tool"
                        && (matches!(e.event_type, EventType::ToolCall)
                            || matches!(e.event_type, EventType::ToolResult))
                    || pattern_lower == "reasoning" && matches!(e.event_type, EventType::Reasoning)
            })
        });
    }

    filtered
}
