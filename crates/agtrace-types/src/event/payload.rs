use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::tool::ToolCallPayload;

use super::agent_payload::*;

/// Event payload variants
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "content")]
#[serde(rename_all = "snake_case")]
pub enum EventPayload {
    /// 1. User input (Trigger)
    User(UserPayload),

    /// 2. Assistant reasoning/thinking process (Claude thinking, Codex reasoning)
    Reasoning(ReasoningPayload),

    /// 3. Tool execution request (Action Request)
    ///
    /// Note: TokenUsage can be attached as sidecar to this
    ToolCall(ToolCallPayload),

    /// 4. Tool execution result (Action Result)
    ToolResult(ToolResultPayload),

    /// 5. Assistant text response (Final Response)
    ///
    /// Note: TokenUsage can be attached as sidecar to this
    Message(MessagePayload),

    /// 6. Cost information (Sidecar / Leaf Node)
    ///
    /// Not included in context, used for cost calculation
    TokenUsage(TokenUsagePayload),

    /// 7. User-facing system notification (updates, alerts, status changes)
    Notification(NotificationPayload),

    /// 8. Slash command invocation (e.g., /commit, /review-pr)
    SlashCommand(SlashCommandPayload),

    /// 9. Background task queue operation
    QueueOperation(QueueOperationPayload),

    /// 10. A child agent was spawned from this agent's log
    AgentSpawn(AgentSpawnPayload),

    /// 11. Lifecycle transition of an agent (running, idle, completed, killed, ...)
    AgentLifecycle(AgentLifecyclePayload),

    /// 12. Inter-agent message (incoming or outgoing)
    AgentMessage(AgentMessagePayload),

    /// 13. Context compaction boundary
    Compaction(CompactionPayload),

    /// 14. End of an agent turn
    TurnEnd(TurnEndPayload),

    /// 15. Model switch
    ModelChange(ModelChangePayload),

    /// 16. In-log evidence about the context window size
    ContextWindowHint(ContextWindowHintPayload),

    /// 17. Agent attribute (title, name, team, ...) — upserted by key
    AgentAttribute(AgentAttributePayload),

    /// 18. Sub-action executed inside a tool call (Codex exec sub-items)
    ToolSubAction(ToolSubActionPayload),
}

impl EventPayload {
    /// Variant name in PascalCase (e.g. "ToolCall", "AgentSpawn").
    pub fn kind_name(&self) -> &'static str {
        match self {
            EventPayload::User(_) => "User",
            EventPayload::Reasoning(_) => "Reasoning",
            EventPayload::ToolCall(_) => "ToolCall",
            EventPayload::ToolResult(_) => "ToolResult",
            EventPayload::Message(_) => "Message",
            EventPayload::TokenUsage(_) => "TokenUsage",
            EventPayload::Notification(_) => "Notification",
            EventPayload::SlashCommand(_) => "SlashCommand",
            EventPayload::QueueOperation(_) => "QueueOperation",
            EventPayload::AgentSpawn(_) => "AgentSpawn",
            EventPayload::AgentLifecycle(_) => "AgentLifecycle",
            EventPayload::AgentMessage(_) => "AgentMessage",
            EventPayload::Compaction(_) => "Compaction",
            EventPayload::TurnEnd(_) => "TurnEnd",
            EventPayload::ModelChange(_) => "ModelChange",
            EventPayload::ContextWindowHint(_) => "ContextWindowHint",
            EventPayload::AgentAttribute(_) => "AgentAttribute",
            EventPayload::ToolSubAction(_) => "ToolSubAction",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPayload {
    /// User input text
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningPayload {
    /// Reasoning/thinking content
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultPayload {
    /// Tool execution result (text, JSON string, error message, etc.)
    pub output: String,

    /// Logical parent (Tool Call) reference ID
    /// Separate from parent_id (time-series parent) to explicitly identify which call this result belongs to
    pub tool_call_id: Uuid,

    /// Execution success or failure
    #[serde(default)]
    pub is_error: bool,

    /// Agent ID if this result spawned a subagent (e.g., "be466c0a")
    /// Used to link sidechain sessions back to their parent turn/step.
    ///
    /// Legacy linkage: superseded by `AgentSpawn` + `AgentRef.spawn_call_id`,
    /// removed once the session assembler no longer needs it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagePayload {
    /// Response text
    pub text: String,
    /// Provider message phase (Codex assistant `phase`, e.g. "commentary", "final_answer")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
}

impl MessagePayload {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            phase: None,
        }
    }
}

// ============================================================================
// Token Usage Normalization
// ============================================================================
//
// Input:  total_input = uncached + cache_read + cache_write
//   - Claude: uncached = input_tokens, cache_read = cache_read_input_tokens,
//             cache_write = cache_creation_input_tokens
//   - Codex:  uncached = input_tokens - cached_input_tokens,
//             cache_read = cached_input_tokens, cache_write = cache_write_input_tokens
//
// Output: total_output = generated + reasoning + tool
//   - Claude: reasoning = output_tokens_details.thinking_tokens, generated = rest
//   - Codex:  generated = output_tokens, reasoning = reasoning_output_tokens
//
// Billing/pricing is out of scope.

/// Input token breakdown
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct TokenInput {
    /// Fresh tokens processed without cache
    pub uncached: u64,
    /// Tokens read from cache (still consume context window)
    #[serde(alias = "cached")]
    pub cache_read: u64,
    /// Tokens written to cache (still consume context window)
    #[serde(default)]
    pub cache_write: u64,
}

impl TokenInput {
    pub fn new(uncached: u64, cache_read: u64, cache_write: u64) -> Self {
        Self {
            uncached,
            cache_read,
            cache_write,
        }
    }

    pub fn total(&self) -> u64 {
        self.uncached + self.cache_read + self.cache_write
    }
}

/// Output token breakdown (generated vs reasoning vs tool)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct TokenOutput {
    /// Normal text generation (assistant messages)
    pub generated: u64,
    /// Reasoning/thinking tokens (extended thinking, o1-style)
    pub reasoning: u64,
    /// Tool call tokens (function calls, structured output)
    pub tool: u64,
}

impl TokenOutput {
    pub fn new(generated: u64, reasoning: u64, tool: u64) -> Self {
        Self {
            generated,
            reasoning,
            tool,
        }
    }

    pub fn total(&self) -> u64 {
        self.generated + self.reasoning + self.tool
    }
}

/// Whether a usage record is final.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum UsageCompleteness {
    #[default]
    Final,
    /// Stream-start snapshot: input/cache exact, output must not be summed as final.
    PartialOutput,
}

impl UsageCompleteness {
    pub fn is_final(&self) -> bool {
        matches!(self, UsageCompleteness::Final)
    }
}

/// Normalized token usage across all providers
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct TokenUsagePayload {
    pub input: TokenInput,
    pub output: TokenOutput,
    /// Model that produced this usage (message.model / current turn model)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Provider key identifying the request (Claude message.id; Codex response_id)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dedupe_key: Option<String>,
    #[serde(default, skip_serializing_if = "UsageCompleteness::is_final")]
    pub completeness: UsageCompleteness,
}

impl TokenUsagePayload {
    pub fn new(input: TokenInput, output: TokenOutput) -> Self {
        Self {
            input,
            output,
            ..Default::default()
        }
    }

    pub fn with_model(mut self, model: Option<String>) -> Self {
        self.model = model;
        self
    }

    pub fn with_dedupe_key(mut self, key: Option<String>) -> Self {
        self.dedupe_key = key;
        self
    }

    pub fn with_completeness(mut self, completeness: UsageCompleteness) -> Self {
        self.completeness = completeness;
        self
    }

    pub fn total_tokens(&self) -> u64 {
        self.input.total() + self.output.total()
    }

    /// Tokens occupying the context window for the request (= input total).
    pub fn context_tokens(&self) -> u64 {
        self.input.total()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPayload {
    /// Notification message text
    pub text: String,
    /// Optional severity level (e.g., "info", "warning", "error")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
    /// Provider subtype (e.g., "turn_duration", "api_error", "pr_link") for filtering
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

/// Slash command invocation (e.g., /commit, /review-pr, /skaffold-repo)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashCommandPayload {
    /// Command name with leading slash (e.g., "/commit", "/skaffold-repo")
    pub name: String,
    /// Optional command arguments
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<String>,
}

/// Background task queue operation (enqueue/dequeue)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueOperationPayload {
    /// Operation type (e.g., "enqueue", "dequeue")
    pub operation: String,
    /// Task content description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Task identifier
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    /// Why the operation happened (e.g., "absorbed_mid_turn")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}
