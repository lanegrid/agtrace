// Engine module - Core processing logic (interpretation, analysis, export)
// This layer sits between normalized events (types) and CLI presentation

pub mod analysis;
pub mod context;
pub mod diagnostics;
pub mod domain;
pub mod export;
pub mod session;
pub mod workspace;

pub use analysis::{SessionDigest, analyze_and_select_sessions};
pub use context::{ContextEvidence, resolve as resolve_context_window};
pub use diagnostics::{DiagnoseResult, FailureExample, FailureType, categorize_parse_error};
pub use domain::{EventFilters, filter_events};
pub use session::{
    AgentSession, AgentStep, AgentTurn, MessageBlock, ReasoningBlock, SessionAnalysisExt,
    SessionStats, SessionSummary, ToolCallBlock, ToolExecution, ToolResultBlock, TurnMetrics,
    TurnStats, UserMessage, assemble_session, assemble_sessions,
};

// Re-export from types for convenience
pub use agtrace_types::{
    CacheCreationTokens, CacheReadTokens, ContextLimit, ContextSource, ContextWindow,
    ContextWindowUsage, FreshInputTokens, ModelCatalog, OutputTokens, TokenCount,
};
