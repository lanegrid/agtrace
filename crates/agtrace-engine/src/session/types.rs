//! Re-export session types from agtrace-types.
//!
//! All session-related types have been moved to agtrace-types to improve
//! architecture clarity and reduce dependency weight for consumers that
//! only need data structures without assembly logic.

pub use agtrace_types::{
    AgentSession, AgentStep, AgentTurn, MessageBlock, ReasoningBlock, SessionStats, StepStatus,
    ToolCallBlock, ToolExecution, ToolResultBlock, TurnMetrics, TurnStats, UserMessage,
};
