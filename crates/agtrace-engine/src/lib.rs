// Engine module - Core processing logic (interpretation, analysis, export)
// This layer sits between normalized events (types) and CLI presentation

pub mod analysis;
pub mod export;
pub mod session;
pub mod span;
pub mod summary;

pub use session::{
    AgentSession, AgentStep, AgentTurn, MessageBlock, ReasoningBlock, SessionStats, ToolCallBlock,
    ToolExecution, ToolResultBlock, TurnStats, UserMessage,
};
pub use span::{build_spans, Message, Span, SpanStats, SystemEvent, TokenBundle, ToolAction};
pub use summary::{summarize, SessionSummary};

// Façade API - Stable public interface for CLI layer
// CLI should use these functions instead of directly accessing internal modules

/// Build spans from events - improved tool matching and token tracking
pub fn build_spans_from_events(events: &[agtrace_types::v2::AgentEvent]) -> Vec<Span> {
    span::build_spans(events)
}

/// Summarize session statistics from events
pub fn summarize_session(events: &[agtrace_types::v2::AgentEvent]) -> SessionSummary {
    summary::summarize(events)
}
