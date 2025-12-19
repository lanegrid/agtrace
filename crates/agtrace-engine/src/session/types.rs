use agtrace_types::{
    MessagePayload, ReasoningPayload, TokenUsagePayload, ToolCallPayload, ToolResultPayload,
    UserPayload,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ==========================================
// 1. Session (entire conversation)
// ==========================================
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSession {
    pub session_id: Uuid,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,

    pub turns: Vec<AgentTurn>,

    pub stats: SessionStats,
}

// ==========================================
// 2. Turn (user-initiated interaction unit)
// ==========================================
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTurn {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,

    /// Turn trigger (Input)
    pub user: UserMessage,

    /// Agent autonomous operation cycle (Steps)
    /// Single step for simple conversation, multiple steps for autonomous agents
    pub steps: Vec<AgentStep>,

    pub stats: TurnStats,
}

// ==========================================
// 3. Step (single LLM inference + execution unit)
// ==========================================
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStep {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,

    // --- Phase 1: Generation (Agent Outputs) ---
    // These are generated in parallel or in arbitrary order before seeing tool results
    /// Reasoning (CoT)
    pub reasoning: Option<ReasoningBlock>,

    /// Text message (answer to user, or declaration of tool execution)
    pub message: Option<MessageBlock>,

    // --- Phase 2: Execution (System Outputs) ---
    /// Tool execution pairs (Call + Result)
    /// Calls are generated in Phase 1, but managed here as pairs with Results
    pub tools: Vec<ToolExecution>,

    // --- Meta ---
    pub usage: Option<TokenUsagePayload>,
    pub is_failed: bool,
}

// ==========================================
// Components
// ==========================================

/// Single tool execution unit (Call -> Result)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecution {
    pub call: ToolCallBlock,

    /// Execution result (None if incomplete or lost)
    pub result: Option<ToolResultBlock>,

    /// Latency (result.timestamp - call.timestamp)
    pub duration_ms: Option<i64>,

    /// Whether this individual tool execution failed
    pub is_error: bool,
}

// --- ID Wrappers ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessage {
    pub event_id: Uuid,
    pub content: UserPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningBlock {
    pub event_id: Uuid,
    pub content: ReasoningPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageBlock {
    pub event_id: Uuid,
    pub content: MessagePayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallBlock {
    pub event_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub provider_call_id: Option<String>,
    pub content: ToolCallPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultBlock {
    pub event_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub tool_call_id: Uuid,
    pub content: ToolResultPayload,
}

// --- Stats ---

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionStats {
    pub total_turns: usize,
    pub duration_seconds: i64,
    pub total_tokens: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TurnStats {
    pub duration_ms: i64,
    pub step_count: usize,
    pub total_tokens: i32,
}
