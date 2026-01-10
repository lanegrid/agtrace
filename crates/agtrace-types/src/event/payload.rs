use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::tool::ToolCallPayload;

/// Event payload variants
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "content")]
#[serde(rename_all = "snake_case")]
pub enum EventPayload {
    /// 1. User input (Trigger)
    User(UserPayload),

    /// 2. Assistant reasoning/thinking process (Gemini thoughts, etc.)
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
    /// Used to link sidechain sessions back to their parent turn/step
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagePayload {
    /// Response text
    pub text: String,
}

// ============================================================================
// Token Usage Normalization
// ============================================================================
//
// # Design Rationale
//
// This normalized token usage schema unifies diverse provider formats into a
// consistent structure based on verified specifications and code behavior.
//
// ## Input Token Normalization
//
// All three providers (Claude, Codex, Gemini) support the decomposition:
//
//   total_input = cached + uncached
//
// **Provider Mappings:**
// - Claude:  cached = cache_read_input_tokens, uncached = input_tokens
// - Codex:   cached = cached_input_tokens, uncached = input_tokens - cached_input_tokens
// - Gemini:  cached = cached, uncached = input
//
// **Specification Guarantee:**
// This relationship is explicitly defined in each provider's API/implementation:
// - Claude: API documentation and usage fields
// - Codex: codex-rs `non_cached_input()` implementation
// - Gemini: gemini-cli telemetry calculation
//
// ## Output Token Normalization
//
// All three providers internally distinguish between token types:
//
//   total_output = generated + reasoning + tool
//
// **Provider Mappings:**
// - Claude:  generated = output_tokens, reasoning = 0*, tool = 0*
// - Codex:   generated = output_tokens, reasoning = reasoning_output_tokens, tool = 0
// - Gemini:  generated = output, reasoning = thoughts, tool = tool
//
// *Note: Claude's content[].type allows parsing reasoning/tool separately (not yet implemented)
//
// **Specification Guarantee:**
// - Codex: Explicit reasoning_output_tokens field in TokenUsage
// - Gemini: Separate thoughts and tool fields in TokenUsage
// - Claude: message.content[].type distinguishes "thinking" and "tool_use"
//
// ## What This Schema Does NOT Track
//
// - **Billing/Pricing**: Token costs vary by provider and usage tier
// - **Cache Creation**: Not uniformly tracked across providers
// - **Visibility**: Whether tokens appear in UI (e.g., hidden reasoning)
//
// This schema focuses solely on **observable token accounting** as reported
// by each provider, ensuring consistent cross-provider analysis.

/// Input token breakdown (cached vs uncached)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct TokenInput {
    /// Tokens read from cache (still consume context window)
    pub cached: u64,
    /// Fresh tokens processed without cache
    pub uncached: u64,
}

impl TokenInput {
    pub fn new(cached: u64, uncached: u64) -> Self {
        Self { cached, uncached }
    }

    pub fn total(&self) -> u64 {
        self.cached + self.uncached
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

/// Normalized token usage across all providers
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct TokenUsagePayload {
    pub input: TokenInput,
    pub output: TokenOutput,
}

impl TokenUsagePayload {
    pub fn new(input: TokenInput, output: TokenOutput) -> Self {
        Self { input, output }
    }

    pub fn total_tokens(&self) -> u64 {
        self.input.total() + self.output.total()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPayload {
    /// Notification message text
    pub text: String,
    /// Optional severity level (e.g., "info", "warning", "error")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
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
