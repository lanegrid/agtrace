#![allow(clippy::format_in_format_args)] // Intentional for colored terminal output

use crate::display_model::{DisplayOptions, SessionDisplay};
use crate::session_loader::{LoadOptions, SessionLoader};
use crate::types::ViewStyle;
use crate::views::session::print_events_timeline;
use agtrace_engine::assemble_session_from_events;
use agtrace_index::Database;
use agtrace_types::v2::{AgentEvent, EventPayload};
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
    style: ViewStyle,
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
    } else {
        match style {
            ViewStyle::Compact => {
                if let Some(session) = assemble_session_from_events(&filtered_events) {
                    let display = SessionDisplay::from_agent_session(&session);
                    let opts = DisplayOptions {
                        enable_color,
                        relative_time: true,
                        truncate_text: if short { Some(100) } else { None },
                    };
                    let lines = crate::views::session::format_compact(&display, &opts);
                    for line in lines {
                        println!("{}", line);
                    }
                } else {
                    eprintln!("Failed to assemble session from events");
                }
            }
            ViewStyle::Timeline => {
                let truncate = short;
                print_events_timeline(&filtered_events, truncate, enable_color);
            }
        }
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
                    EventPayload::Notification(_) => {
                        pattern_lower == "notification" || pattern_lower == "info"
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
                    EventPayload::Notification(_) => {
                        pattern_lower == "notification" || pattern_lower == "info"
                    }
                }
            })
        });
    }

    filtered
}
