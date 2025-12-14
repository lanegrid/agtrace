// Engine module - Core processing logic (interpretation, analysis, export)
// This layer sits between normalized events (types) and CLI presentation

pub mod analysis;
pub mod export;
pub mod span;
pub mod span_v2;
pub mod summary;
mod turn;

pub use span::{build_spans, Message, Span, SpanStats, SystemEvent, TokenBundle, ToolAction};
pub use span_v2::build_spans_v2;
pub use summary::SessionSummary;
pub use turn::{
    interpret_turns, ActionResult, ChainItem, SystemMessageKind, Turn, TurnOutcome, TurnStats,
};

use agtrace_types::AgentEventV1;

// Façade API - Stable public interface for CLI layer
// CLI should use these functions instead of directly accessing internal modules

/// Build turns from events - structured conversation representation (v1)
pub fn build_turns(events: &[AgentEventV1]) -> Vec<Turn> {
    interpret_turns(events)
}

/// Summarize session statistics from events (v1)
pub fn summarize_session(events: &[AgentEventV1]) -> SessionSummary {
    summary::summarize(events)
}

// V2 API - New functions for v2 schema

/// Build spans from v2 events - improved tool matching and token tracking
pub fn build_spans_from_v2(events: &[agtrace_types::v2::AgentEvent]) -> Vec<Span> {
    span_v2::build_spans_v2(events)
}
