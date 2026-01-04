// Engine module - Core processing logic (interpretation, analysis, export)
// This layer sits between normalized events (types) and CLI presentation

pub mod analysis;
pub mod diagnostics;
pub mod domain;
pub mod export;
pub mod session;
pub mod state_updates;

pub use analysis::{SessionDigest, analyze_and_select_sessions};
pub use diagnostics::{DiagnoseResult, FailureExample, FailureType, categorize_parse_error};
pub use domain::{EventFilters, SessionState, TokenLimit, TokenLimits, filter_events};
pub use session::{
    AgentSession, AgentStep, AgentTurn, MessageBlock, ReasoningBlock, SessionAnalysisExt,
    SessionStats, SessionSummary, ToolCallBlock, ToolExecution, ToolResultBlock, TurnMetrics,
    TurnStats, UserMessage, assemble_session, assemble_sessions,
};
pub use state_updates::{StateUpdates, extract_state_updates};

// Re-export from types for convenience
pub use agtrace_types::{
    CacheCreationTokens, CacheReadTokens, ContextLimit, ContextWindowUsage, FreshInputTokens,
    ModelLimitResolver, ModelSpec, OutputTokens, TokenCount,
};
