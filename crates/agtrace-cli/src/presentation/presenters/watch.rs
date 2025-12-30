use std::collections::VecDeque;
use std::path::Path;

use crate::presentation::view_models::{WatchEventViewModel, WatchTargetViewModel};

/// Present watch start event
pub fn present_watch_start_provider(
    provider_name: String,
    log_root: impl AsRef<Path>,
) -> WatchEventViewModel {
    WatchEventViewModel::Start {
        target: WatchTargetViewModel::Provider {
            name: provider_name,
            log_root: log_root.as_ref().to_path_buf(),
        },
    }
}

/// Present watch start event for session
pub fn present_watch_start_session(
    session_id: String,
    log_root: impl AsRef<Path>,
) -> WatchEventViewModel {
    WatchEventViewModel::Start {
        target: WatchTargetViewModel::Session {
            id: session_id,
            log_root: log_root.as_ref().to_path_buf(),
        },
    }
}

/// Present session attached event
pub fn present_watch_attached(session_id: String) -> WatchEventViewModel {
    WatchEventViewModel::Attached { session_id }
}

/// Present session rotated event
pub fn present_watch_rotated(old_session: String, new_session: String) -> WatchEventViewModel {
    WatchEventViewModel::Rotated {
        old_session,
        new_session,
    }
}

/// Present waiting event
pub fn present_watch_waiting(message: String) -> WatchEventViewModel {
    WatchEventViewModel::Waiting { message }
}

/// Present error event
pub fn present_watch_error(message: String, fatal: bool) -> WatchEventViewModel {
    WatchEventViewModel::Error { message, fatal }
}

/// Present stream update event using unified screen view model
///
/// Converts domain data (SessionState + Events + Session) into WatchEventViewModel
/// using the same presenter logic as TUI mode
pub fn present_watch_stream_update(
    state: &agtrace_sdk::types::SessionState,
    events: &VecDeque<agtrace_sdk::types::AgentEvent>,
    assembled_session: Option<&agtrace_sdk::types::AgentSession>,
    max_context: Option<u32>,
    notification: Option<&str>,
) -> WatchEventViewModel {
    use super::watch_tui::build_screen_view_model;

    // Use the same unified presenter as TUI
    let screen =
        build_screen_view_model(state, events, assembled_session, max_context, notification);

    WatchEventViewModel::StreamUpdate {
        screen: Box::new(screen),
    }
}
