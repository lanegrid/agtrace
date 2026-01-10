use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::project::ProjectHash;
use super::token_usage::ContextWindowUsage;
use crate::{
    MessagePayload, ReasoningPayload, SlashCommandPayload, StreamId, ToolCallPayload,
    ToolResultPayload, UserPayload,
};

/// Source of the agent log (provider-agnostic identifier)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Source(String);

impl Source {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

/// Subagent execution information
///
/// Represents metadata about subagent (agent-within-agent) execution.
/// Different providers implement subagents differently:
/// - Claude Code: Uses Task tool with `subagent_type` and returns `agentId`
/// - Codex: Creates separate session files with `source.subagent` metadata
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubagentInfo {
    /// Subagent identifier (e.g., "ba2ed465" for Claude Code, session ID for Codex)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,

    /// Subagent type/role (e.g., "Explore", "general-purpose", "review")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_type: Option<String>,

    /// Parent session ID (for Codex where subagent is a separate session)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_session_id: Option<String>,
}

/// Tool execution status (used in Span API)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolStatus {
    Success,
    Error,
    InProgress,
    Unknown,
}

/// Order for session listing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionOrder {
    /// Most recent first (end_ts DESC, start_ts DESC)
    NewestFirst,
    /// Oldest first (start_ts ASC, end_ts ASC)
    OldestFirst,
}

impl Default for SessionOrder {
    fn default() -> Self {
        Self::NewestFirst
    }
}

/// Session summary for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub session_id: String,
    pub source: Source,
    pub project_hash: ProjectHash,
    pub start_ts: String,
    pub end_ts: String,
    pub event_count: usize,
    pub user_message_count: usize,
    pub tokens_input_total: u64,
    pub tokens_output_total: u64,
}

/// Session metadata (DB-derived, not available from events alone).
///
/// Contains information inferred from filesystem paths and stored in the index.
/// Separate from AgentSession which is assembled purely from events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    /// Unique session identifier.
    pub session_id: String,
    /// Project hash (inferred from log file path).
    pub project_hash: ProjectHash,
    /// Project root path (resolved from project_hash).
    pub project_root: Option<String>,
    /// Provider name (claude_code, codex, gemini).
    pub provider: String,
    /// Parent session ID for subagent sessions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_session_id: Option<String>,
    /// Spawn context for subagent sessions (turn/step where spawned).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spawned_by: Option<SpawnContext>,
}

// ==========================================
// 1. Session (entire conversation)
// ==========================================

/// Context about how a sidechain was spawned from a parent session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnContext {
    /// Turn index (0-based) in the parent session where this sidechain was spawned.
    pub turn_index: usize,
    /// Step index (0-based) within the turn where the Task tool was called.
    pub step_index: usize,
}

/// Complete agent conversation session assembled from normalized events.
///
/// Represents a full conversation timeline with the agent, containing all
/// user interactions (turns) and their corresponding agent responses.
/// The session is the highest-level construct for analyzing agent behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSession {
    /// Unique session identifier.
    pub session_id: Uuid,
    /// Stream identifier for multi-stream sessions.
    /// Indicates whether this is the main conversation, a sidechain, or a subagent.
    pub stream_id: StreamId,
    /// For sidechain sessions: context about where this was spawned from in the parent session.
    /// None for main stream sessions or sidechains without identifiable parent context.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spawned_by: Option<SpawnContext>,
    /// When the session started (first event timestamp).
    pub start_time: DateTime<Utc>,
    /// When the session ended (last event timestamp), if completed.
    pub end_time: Option<DateTime<Utc>>,

    /// All user-initiated turns in chronological order.
    pub turns: Vec<AgentTurn>,

    /// Aggregated session statistics.
    pub stats: SessionStats,
}

// ==========================================
// 2. Turn (user-initiated interaction unit)
// ==========================================

/// Single user-initiated interaction cycle within a session.
///
/// A turn begins with user input and contains all agent steps taken
/// in response until the next user input or session end.
/// Autonomous agents may execute multiple steps per turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTurn {
    /// Unique turn identifier (ID of the user event that initiated this turn).
    pub id: Uuid,
    /// When the turn started (user input timestamp).
    pub timestamp: DateTime<Utc>,

    /// User input that triggered this turn.
    pub user: UserMessage,

    /// Agent's response steps in chronological order.
    /// Single step for simple Q&A, multiple steps for autonomous operation.
    pub steps: Vec<AgentStep>,

    /// Aggregated turn statistics.
    pub stats: TurnStats,
}

// ==========================================
// 3. Step (single LLM inference + execution unit)
// ==========================================

/// Single LLM inference cycle with optional tool executions.
///
/// A step represents one round of agent thinking and acting:
/// 1. Generation phase: LLM produces reasoning, messages, and tool calls
/// 2. Execution phase: Tools are executed and results collected
///
/// Steps are the atomic unit of agent behavior analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStep {
    /// Unique step identifier (ID of the first event in this step).
    pub id: Uuid,
    /// When the step started.
    pub timestamp: DateTime<Utc>,

    // --- Phase 1: Generation (Agent Outputs) ---
    // These are generated in parallel or in arbitrary order before seeing tool results
    /// Chain-of-thought reasoning, if present.
    pub reasoning: Option<ReasoningBlock>,

    /// Text response to user or explanation of tool usage.
    pub message: Option<MessageBlock>,

    // --- Phase 2: Execution (System Outputs) ---
    /// Tool executions (call + result pairs) performed in this step.
    /// Calls are generated in Phase 1, paired with results here.
    pub tools: Vec<ToolExecution>,

    // --- Meta ---
    /// Token usage for this step's LLM inference, if available.
    pub usage: Option<ContextWindowUsage>,
    /// Whether this step encountered any failures.
    pub is_failed: bool,
    /// Current completion status of this step.
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

/// Paired tool call and result with execution metrics.
///
/// Represents a complete tool execution lifecycle:
/// tool invocation → execution → result collection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecution {
    /// Tool invocation request.
    pub call: ToolCallBlock,

    /// Execution result (None if incomplete, lost, or still pending).
    pub result: Option<ToolResultBlock>,

    /// Execution latency in milliseconds (result.timestamp - call.timestamp).
    pub duration_ms: Option<i64>,

    /// Whether this tool execution failed (error status in result).
    pub is_error: bool,
}

// --- Event Wrappers ---

/// User input message that initiates a turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessage {
    /// ID of the source event.
    pub event_id: Uuid,
    /// User input content (empty if triggered by slash command).
    pub content: UserPayload,
    /// Slash command that triggered this turn (e.g., /commit, /skaffold-repo).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slash_command: Option<SlashCommandPayload>,
}

/// Agent reasoning/thinking block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningBlock {
    /// ID of the source event.
    pub event_id: Uuid,
    /// Reasoning content.
    pub content: ReasoningPayload,
}

/// Agent text response message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageBlock {
    /// ID of the source event.
    pub event_id: Uuid,
    /// Message content.
    pub content: MessagePayload,
}

/// Tool invocation request with timing information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallBlock {
    /// ID of the source event.
    pub event_id: Uuid,
    /// When the tool was invoked.
    pub timestamp: DateTime<Utc>,
    /// Provider-specific call identifier, if available.
    pub provider_call_id: Option<String>,
    /// Tool invocation details.
    pub content: ToolCallPayload,
}

/// Tool execution result with timing information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultBlock {
    /// ID of the source event.
    pub event_id: Uuid,
    /// When the result was received.
    pub timestamp: DateTime<Utc>,
    /// ID of the tool call this result corresponds to.
    pub tool_call_id: Uuid,
    /// Tool execution result details.
    pub content: ToolResultPayload,
}

// --- Stats ---

/// Aggregated statistics for an entire session.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionStats {
    /// Total number of turns in the session.
    pub total_turns: usize,
    /// Session duration in seconds (end_time - start_time).
    pub duration_seconds: i64,
    /// Total tokens consumed across all turns.
    pub total_tokens: i64,
}

/// Aggregated statistics for a single turn.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TurnStats {
    /// Turn duration in milliseconds.
    pub duration_ms: i64,
    /// Number of steps in this turn.
    pub step_count: usize,
    /// Total tokens consumed in this turn.
    pub total_tokens: i32,
}

// ==========================================
// Computed Metrics (for presentation)
// ==========================================

/// Computed context window metrics for turn visualization.
///
/// Used by TUI and other presentation layers to show
/// cumulative token usage and detect high-usage patterns.
#[derive(Debug, Clone)]
pub struct TurnMetrics {
    /// Zero-based turn index.
    pub turn_index: usize,
    /// Cumulative tokens before this turn (0 if context was compacted).
    pub prev_total: u32,
    /// Tokens added by this turn (or new baseline if compacted).
    pub delta: u32,
    /// Whether this turn's delta exceeds the heavy threshold.
    pub is_heavy: bool,
    /// Whether this turn is currently active (in progress).
    pub is_active: bool,
    /// True if context was compacted (reset) during this turn.
    pub context_compacted: bool,
    /// Actual cumulative tokens at end of this turn.
    pub cumulative_total: u32,
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
            .map(|usage| usage.input_tokens() as u32)
            .unwrap_or(fallback)
    }

    /// Calculate cumulative total tokens (input + output) at the end of this turn
    /// Falls back to `fallback` if no usage data found
    pub fn cumulative_total_tokens(&self, fallback: u32) -> u32 {
        self.steps
            .iter()
            .rev()
            .find_map(|step| step.usage.as_ref())
            .map(|usage| (usage.input_tokens() + usage.output_tokens()) as u32)
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
