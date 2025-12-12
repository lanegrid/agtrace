// Engine module - Core processing logic (interpretation, analysis, export)
// This layer sits between normalized events (types) and CLI presentation

pub mod analysis;
pub mod export;
pub mod summary;
mod turn;

pub use summary::SessionSummary;
pub use turn::{
    interpret_turns, ActionResult, ChainItem, SystemMessageKind, Turn, TurnOutcome, TurnStats,
};

use agtrace_types::AgentEventV1;

// Façade API - Stable public interface for CLI layer
// CLI should use these functions instead of directly accessing internal modules

/// Build turns from events - structured conversation representation
pub fn build_turns(events: &[AgentEventV1]) -> Vec<Turn> {
    interpret_turns(events)
}

/// Summarize session statistics from events
pub fn summarize_session(events: &[AgentEventV1]) -> SessionSummary {
    summary::summarize(events)
}
