// Engine module - Core processing logic (interpretation, analysis, export)
// This layer sits between normalized events (types) and CLI presentation

mod activity;
pub mod analysis;
pub mod export;
pub mod summary;

pub use activity::{
    interpret_events, interpret_events_with_options, Activity, ActivityStats, ActivityStatus,
    InterpretOptions, ToolSummary,
};
pub use summary::SessionSummary;

use agtrace_types::AgentEventV1;

// Façade API - Stable public interface for CLI layer
// CLI should use these functions instead of directly accessing internal modules

/// Build activities from events with options for display control
pub fn build_activities(events: &[AgentEventV1], opts: &InterpretOptions) -> Vec<Activity> {
    interpret_events_with_options(events, opts)
}

/// Summarize session statistics from events
pub fn summarize_session(events: &[AgentEventV1]) -> SessionSummary {
    summary::summarize(events)
}
