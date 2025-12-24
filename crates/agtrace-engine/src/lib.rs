// Engine module - Core processing logic (interpretation, analysis, export)
// This layer sits between normalized events (types) and CLI presentation

pub mod analysis;
pub mod diagnostics;
pub mod export;
pub mod session;
pub mod state_updates;
pub mod token_usage;

pub use analysis::{analyze_and_select_sessions, SessionDigest};
pub use diagnostics::{categorize_parse_error, DiagnoseResult, FailureExample, FailureType};
pub use session::{
    assemble_session, AgentSession, AgentStep, AgentTurn, MessageBlock, ReasoningBlock,
    SessionStats, SessionSummary, ToolCallBlock, ToolExecution, ToolResultBlock, TurnMetrics,
    TurnStats, UserMessage,
};
pub use state_updates::{extract_state_updates, StateUpdates};
pub use token_usage::{
    CacheCreationTokens, CacheReadTokens, ContextLimit, ContextWindowUsage, FreshInputTokens,
    OutputTokens, TokenCount,
};
