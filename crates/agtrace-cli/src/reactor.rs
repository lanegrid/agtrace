use agtrace_types::v2::AgentEvent;
use anyhow::Result;
use chrono::{DateTime, Utc};
use std::path::PathBuf;

/// Reaction returned by reactors to instruct the main loop
#[derive(Debug, Clone)]
pub enum Reaction {
    /// Continue processing normally
    Continue,

    /// Warning level - logs warning but continues
    Warn(String),

    /// Intervention request - requires action from main loop
    Intervene { reason: String, severity: Severity },
}

/// Severity level for interventions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Low severity - desktop notification or console alert
    Notification,

    /// High severity - terminate the agent process (future: v0.2.0)
    Kill,
}

/// Lightweight session state metadata for reactor context
/// This is NOT the full AgentSession, but a summary of key metrics
#[derive(Debug, Clone)]
pub struct SessionState {
    /// Session/trace ID
    pub session_id: String,

    /// Project root path
    pub project_root: Option<PathBuf>,

    /// Session start time
    pub start_time: DateTime<Utc>,

    /// Last activity timestamp
    pub last_activity: DateTime<Utc>,

    /// Total input tokens consumed
    pub total_input_tokens: i32,

    /// Total output tokens consumed
    pub total_output_tokens: i32,

    /// Consecutive error count (reset on success)
    pub error_count: u32,

    /// Total number of events processed
    pub event_count: usize,

    /// Number of turns (user inputs)
    pub turn_count: usize,
}

impl SessionState {
    /// Create initial state from first event
    pub fn new(
        session_id: String,
        project_root: Option<PathBuf>,
        start_time: DateTime<Utc>,
    ) -> Self {
        Self {
            session_id,
            project_root,
            start_time,
            last_activity: start_time,
            total_input_tokens: 0,
            total_output_tokens: 0,
            error_count: 0,
            event_count: 0,
            turn_count: 0,
        }
    }
}

/// Context passed to reactors for each event
#[derive(Clone, Copy)]
pub struct ReactorContext<'a> {
    /// Current event (trigger)
    pub event: &'a AgentEvent,

    /// Session state snapshot (background context)
    pub state: &'a SessionState,
}

/// Reactor trait - pluggable event handlers
pub trait Reactor: Send {
    /// Reactor name for debugging
    fn name(&self) -> &str;

    /// Handle an event and return a reaction
    /// This is called synchronously for each event in the main loop
    fn handle(&mut self, ctx: ReactorContext) -> Result<Reaction>;
}
