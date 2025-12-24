// Engine module - Core processing logic (interpretation, analysis, export)
// This layer sits between normalized events (types) and CLI presentation

pub mod analysis;
pub mod diagnostics;
pub mod export;
pub mod session;
pub mod state_updates;
pub mod token_usage;

pub use analysis::{SessionDigest, analyze_and_select_sessions};
pub use diagnostics::{DiagnoseResult, FailureExample, FailureType, categorize_parse_error};
pub use session::{
    AgentSession, AgentStep, AgentTurn, MessageBlock, ReasoningBlock, SessionStats, SessionSummary,
    ToolCallBlock, ToolExecution, ToolResultBlock, TurnMetrics, TurnStats, UserMessage,
    assemble_session,
};
pub use state_updates::{StateUpdates, extract_state_updates};
pub use token_usage::{
    CacheCreationTokens, CacheReadTokens, ContextLimit, ContextWindowUsage, FreshInputTokens,
    OutputTokens, TokenCount,
};
