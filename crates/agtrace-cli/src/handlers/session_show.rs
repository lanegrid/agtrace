#![allow(clippy::format_in_format_args)] // Intentional for colored terminal output

use crate::presentation::formatters::DisplayOptions;
use crate::presentation::presenters;
use crate::presentation::renderers::TraceView;
use crate::presentation::view_models::RawFileContent;
use crate::types::ViewStyle;
use agtrace_engine::assemble_session;
use agtrace_index::Database;
use agtrace_runtime::{LoadOptions, SessionRepository};
use agtrace_types::{AgentEvent, EventPayload};
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
    hide: Option<Vec<String>>,
    only: Option<Vec<String>>,
    short: bool,
    verbose: bool,
    view: &dyn TraceView,
) -> Result<()> {
    // Detect if output is being piped (not a terminal)
    let is_tty = io::stdout().is_terminal();
    let enable_color = is_tty;

    // Handle raw mode (display raw files without normalization)
    if raw {
        let log_files = db.get_session_files(&session_id)?;

        let mut contents = Vec::new();

        for log_file in &log_files {
            let content = fs::read_to_string(&log_file.path)
                .with_context(|| format!("Failed to read file: {}", log_file.path))?;
            contents.push(RawFileContent {
                path: log_file.path.clone(),
                content,
            });
        }
        view.render_session_raw_files(&contents)?;
        return Ok(());
    }

    // Load and normalize events
    let loader = SessionRepository::new(db);
    let options = LoadOptions::default();
    let all_events = loader.load_events(&session_id, &options)?;

    // Filter events based on --hide and --only options
    let filtered_events = filter_events(&all_events, hide.as_ref(), only.as_ref());

    if json {
        let event_vms = presenters::present_events(&filtered_events);
        view.render_session_events_json(&event_vms)?;
    } else {
        let style = if verbose {
            ViewStyle::Timeline
        } else {
            ViewStyle::Compact
        };

        match style {
            ViewStyle::Compact => {
                if let Some(session) = assemble_session(&filtered_events) {
                    let session_vm = presenters::present_session(&session);
                    let opts = DisplayOptions {
                        enable_color,
                        relative_time: true,
                        truncate_text: if short { Some(100) } else { None },
                    };
                    view.render_session_compact(&session_vm, &opts)?;
                } else {
                    view.render_session_assemble_error()?;
                }
            }
            ViewStyle::Timeline => {
                let truncate = short;
                let event_vms = presenters::present_events(&filtered_events);
                view.render_session_timeline(&event_vms, truncate, enable_color)?;
            }
        }
    }

    Ok(())
}

/// Filter events based on hide/only patterns
fn filter_events(
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
