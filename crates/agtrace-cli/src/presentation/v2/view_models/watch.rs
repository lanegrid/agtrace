use chrono::{DateTime, Utc};
use serde::Serialize;
use std::path::PathBuf;

use super::lab::EventViewModel;
use super::session::{ContextWindowUsageViewModel, TurnUsageViewModel};

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
    /// Stream update with new events
    StreamUpdate {
        state: WatchStreamStateViewModel,
        events: Vec<EventViewModel>,
        turns: Option<Vec<TurnUsageViewModel>>,
    },
    /// Error occurred
    Error { message: String, fatal: bool },
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum WatchTargetViewModel {
    Provider { name: String, log_root: PathBuf },
    Session { id: String, log_root: PathBuf },
}

/// Session state snapshot for watch stream
#[derive(Debug, Clone, Serialize)]
pub struct WatchStreamStateViewModel {
    pub session_id: String,
    pub project_root: Option<String>,
    pub start_time: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub model: Option<String>,
    pub event_count: usize,
    pub turn_count: usize,
    pub current_usage: ContextWindowUsageViewModel,
    pub token_limit: Option<u64>,
    pub compaction_buffer_pct: Option<f64>,
}
