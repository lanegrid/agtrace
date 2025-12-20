#![allow(clippy::format_in_format_args)] // Intentional for colored terminal output

use crate::presentation::presenters;
use crate::presentation::renderers::TraceView;
use crate::presentation::view_models::{DisplayOptions, RawFileContent};
use crate::types::ViewStyle;
use agtrace_engine::assemble_session;
use agtrace_index::Database;
use agtrace_runtime::{LoadOptions, SessionRepository};
use anyhow::{Context, Result};
use is_terminal::IsTerminal;
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
        let service = agtrace_runtime::SessionService::new(db);
        let contents = service
            .get_raw_files(&session_id)
            .with_context(|| format!("Failed to load raw files for session: {}", session_id))?;

        let view_contents: Vec<RawFileContent> = contents
            .into_iter()
            .map(|c| RawFileContent {
                path: c.path,
                content: c.content,
            })
            .collect();

        view.render_session_raw_files(&view_contents)?;
        return Ok(());
    }

    // Load and normalize events
    let loader = SessionRepository::new(db);
    let options = LoadOptions::default();
    let all_events = loader.load_events(&session_id, &options)?;

    // Filter events based on --hide and --only options
    let service = agtrace_runtime::SessionService::new(db);
    let filtered_events =
        service.filter_events(&all_events, agtrace_runtime::EventFilters { hide, only });

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
