use serde::Serialize;
use std::path::PathBuf;

use super::watch_tui::TuiScreenViewModel;

// --------------------------------------------------------
// Watch Event ViewModels (Producer/Consumer pattern)
// --------------------------------------------------------

/// Streaming events from watch session
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum WatchEventViewModel {
    /// Watch monitoring started
    Start { target: WatchTargetViewModel },
    /// Attached to a session
    Attached { session_id: String },
    /// Session switched (for provider watch)
    Rotated {
        old_session: String,
        new_session: String,
    },
    /// Waiting for new session
    Waiting { message: String },
    /// Stream update with unified screen view model
    StreamUpdate { screen: Box<TuiScreenViewModel> },
    /// Error occurred
    Error { message: String, fatal: bool },
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum WatchTargetViewModel {
    Provider { name: String, log_root: PathBuf },
    Session { id: String, log_root: PathBuf },
}
