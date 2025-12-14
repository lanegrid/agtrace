// Engine module - Core processing logic (interpretation, analysis, export)
// This layer sits between normalized events (types) and CLI presentation

pub mod assembler;
pub mod export;
pub mod session;
pub mod span;
pub mod summary;

pub use assembler::assemble_session;
pub use session::{
    AgentSession, AgentStep, AgentTurn, MessageBlock, ReasoningBlock, SessionStats, ToolCallBlock,
    ToolExecution, ToolResultBlock, TurnStats, UserMessage,
};
pub use span::{build_spans, Message, Span, SpanStats, SystemEvent, TokenBundle, ToolAction};
pub use summary::{summarize, SessionSummary};

// Façade API - Stable public interface for CLI layer
// CLI should use these functions instead of directly accessing internal modules

/// Assemble events into AgentSession structure
pub fn assemble_session_from_events(
    events: &[agtrace_types::v2::AgentEvent],
) -> Option<AgentSession> {
    assembler::assemble_session(events)
}

/// Build spans from events - improved tool matching and token tracking
pub fn build_spans_from_events(events: &[agtrace_types::v2::AgentEvent]) -> Vec<Span> {
    span::build_spans(events)
}

/// Summarize session statistics from AgentSession
pub fn summarize_session(session: &AgentSession) -> SessionSummary {
    summary::summarize(session)
}
