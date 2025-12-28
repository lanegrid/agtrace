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
    pub status: StepStatus,
}

/// Step completion status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    /// Step completed successfully (has Message or all tools have results)
    Done,
    /// Step is waiting for tool results or next action
    InProgress,
    /// Step failed with errors
    Failed,
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

// ==========================================
// Computed Metrics (for presentation)
// ==========================================

/// Computed metrics for a turn, used for presentation layer
#[derive(Debug, Clone)]
pub struct TurnMetrics {
    pub turn_index: usize,
    pub prev_total: u32,
    pub delta: u32,
    pub is_heavy: bool,
    pub is_active: bool,
}

impl TurnMetrics {
    /// Calculate heavy threshold: 10% of max context, or fallback to 15k tokens
    pub fn heavy_threshold(max_context: Option<u32>) -> u32 {
        max_context.map(|mc| mc / 10).unwrap_or(15000)
    }

    /// Check if a delta is considered heavy
    pub fn is_delta_heavy(delta: u32, max_context: Option<u32>) -> bool {
        delta >= Self::heavy_threshold(max_context)
    }
}

impl AgentTurn {
    /// Calculate cumulative input tokens at the end of this turn
    /// Falls back to `fallback` if no usage data found
    pub fn cumulative_input_tokens(&self, fallback: u32) -> u32 {
        self.steps
            .iter()
            .rev()
            .find_map(|step| step.usage.as_ref())
            .map(|usage| {
                (usage.input_tokens
                    + usage
                        .details
                        .as_ref()
                        .and_then(|d| d.cache_creation_input_tokens)
                        .unwrap_or(0)
                    + usage
                        .details
                        .as_ref()
                        .and_then(|d| d.cache_read_input_tokens)
                        .unwrap_or(0)) as u32
            })
            .unwrap_or(fallback)
    }

    /// Calculate cumulative total tokens (input + output) at the end of this turn
    /// Falls back to `fallback` if no usage data found
    pub fn cumulative_total_tokens(&self, fallback: u32) -> u32 {
        self.steps
            .iter()
            .rev()
            .find_map(|step| step.usage.as_ref())
            .map(|usage| {
                (usage.input_tokens
                    + usage
                        .details
                        .as_ref()
                        .and_then(|d| d.cache_creation_input_tokens)
                        .unwrap_or(0)
                    + usage
                        .details
                        .as_ref()
                        .and_then(|d| d.cache_read_input_tokens)
                        .unwrap_or(0)
                    + usage.output_tokens) as u32
            })
            .unwrap_or(fallback)
    }

    /// Check if this turn is currently active
    ///
    /// A turn is active if any of the recent steps are in progress.
    /// Looking at multiple steps provides stability during step transitions
    /// (e.g., when a step completes but the next one hasn't started yet).
    pub fn is_active(&self) -> bool {
        const LOOKBACK_STEPS: usize = 3;

        self.steps
            .iter()
            .rev()
            .take(LOOKBACK_STEPS)
            .any(|step| matches!(step.status, StepStatus::InProgress))
    }
}

impl AgentSession {
    /// Compute presentation metrics for all turns
    pub fn compute_turn_metrics(&self, max_context: Option<u32>) -> Vec<TurnMetrics> {
        let mut cumulative_total = 0u32;
        let mut metrics = Vec::new();
        let total_turns = self.turns.len();

        for (idx, turn) in self.turns.iter().enumerate() {
            let turn_end_cumulative = turn.cumulative_total_tokens(cumulative_total);
            let delta = turn_end_cumulative.saturating_sub(cumulative_total);
            let prev_total = cumulative_total;

            // Last turn is always active during streaming to avoid flicker
            // when steps transition between InProgress and Done
            let is_active = if idx == total_turns.saturating_sub(1) {
                // For streaming sessions, last turn is active regardless of step status
                // This prevents "CURRENT TURN" display from flickering during step transitions
                true
            } else {
                false
            };

            metrics.push(TurnMetrics {
                turn_index: idx,
                prev_total,
                delta,
                is_heavy: TurnMetrics::is_delta_heavy(delta, max_context),
                is_active,
            });

            cumulative_total = turn_end_cumulative;
        }

        metrics
    }
}
