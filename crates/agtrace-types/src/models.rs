use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

// NOTE: Schema Design Goals
//
// 1. Normalization: Abstract provider-specific quirks into unified time-series events
//    - Gemini: Unfold nested batch records into sequential events
//    - Codex: Align async token notifications and eliminate echo duplicates
//    - Claude: Extract embedded usage into independent events
//
// 2. Observability: Enable accurate cost/performance tracking
//    - Token: Sidecar pattern + incremental detection for precise billing (no double-counting)
//    - Latency: Measure turnaround time (T_req → T_res) from user perspective
//
// 3. Replayability: Reconstruct full conversation context via parent_id chain
//    - Linked-list structure ensures deterministic history recovery regardless of parallel execution
//
// 4. Separation: Distinguish time-series flow (parent_id) from logical relations (tool_call_id)
//    - Enables both "conversation replay" and "request/response mapping"
//
// NOTE: Intentional Limitations (Not Goals)
//
// - OS-level execution timestamps: Unavailable in logs; command issue time ≒ execution start
// - Tree/branch structure: Parallel tool calls are linearized in chronological/array order
// - Real-time token sync: Codex-style delayed tokens handled via eventual consistency (sidecar)
// - Gemini token breakdown: Total usage attached to final generation event (no speculation)

/// Stream identifier for multi-stream sessions
/// Enables parallel conversation streams within same session (e.g., background reasoning, subagents)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(tag = "stream_type", content = "stream_data")]
#[serde(rename_all = "snake_case")]
pub enum StreamId {
    /// Main conversation stream (default)
    #[default]
    Main,
    /// Claude sidechain (background agent with specific ID)
    Sidechain { agent_id: String },
    /// Codex subagent (e.g., "review", "test", etc.)
    Subagent { name: String },
}

impl StreamId {
    /// Get string representation for debugging/logging
    pub fn as_str(&self) -> String {
        match self {
            StreamId::Main => "main".to_string(),
            StreamId::Sidechain { agent_id } => format!("sidechain:{}", agent_id),
            StreamId::Subagent { name } => format!("subagent:{}", name),
        }
    }
}

/// Tool classification by semantic purpose
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolKind {
    /// Read operations (files, resources, data)
    Read,
    /// Write operations (edit, create, patch)
    Write,
    /// Execute operations (shell commands, scripts)
    Execute,
    /// Planning operations (todo, task management)
    Plan,
    /// Search operations (web, file search, grep)
    Search,
    /// User interaction (questions, prompts)
    Ask,
    /// Other/unknown operations
    Other,
}

/// Tool origin classification
///
/// Distinguishes between provider-native tools and MCP protocol tools.
///
/// # Important
/// The origin is determined by how the tool is invoked, not by what it operates on:
/// - `System`: Tool is built-in to the provider and invoked directly by the LLM
/// - `Mcp`: Tool is invoked via MCP protocol (typically prefixed with `mcp__`)
///
/// # Examples
/// - `Bash` (Claude Code) → System (provider-native tool)
/// - `read_mcp_resource` (Codex) → System (provider-native tool that happens to read MCP resources)
/// - `mcp__sqlite__query` → Mcp (external tool invoked via MCP protocol)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolOrigin {
    /// System-provided tool (built-in to the provider)
    System,
    /// MCP (Model Context Protocol) tool invoked via MCP protocol
    Mcp,
}

/// Agent event
/// Maps 1:1 to database table row
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEvent {
    /// Unique event ID
    pub id: Uuid,

    /// Session/trace ID (groups entire conversation)
    pub session_id: Uuid,

    /// Parent event ID in time-series chain (Linked List structure)
    /// None for root events (first User input)
    pub parent_id: Option<Uuid>,

    /// Event timestamp (UTC)
    pub timestamp: DateTime<Utc>,

    /// Stream identifier (main, sidechain, subagent)
    /// Enables parallel conversation streams within same session
    #[serde(default)]
    pub stream_id: StreamId,

    /// Event type and content (flattened enum)
    #[serde(flatten)]
    pub payload: EventPayload,

    /// Provider-specific raw data and debug information
    /// Examples: Codex "call_id", Gemini "finish_reason", etc.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

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
}

// --- Payload Definitions ---

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
pub struct ToolCallPayload {
    /// Tool name to execute (e.g., "bash", "write_file")
    pub name: String,

    /// Tool arguments (JSON Value)
    /// Codex's stringified JSON should be parsed into Value object
    pub arguments: Value,

    /// Provider-assigned call ID (e.g., "call_Dyh...", "toolu_01...")
    /// Essential for linking with ToolResult and log tracing
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_call_id: Option<String>,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagePayload {
    /// Response text
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsagePayload {
    /// Input tokens (incremental)
    pub input_tokens: i32,
    /// Output tokens (incremental)
    pub output_tokens: i32,
    /// Total tokens (incremental)
    pub total_tokens: i32,

    /// Detailed breakdown (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<TokenUsageDetails>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsageDetails {
    /// Cache creation input tokens (Claude/Gemini)
    pub cache_creation_input_tokens: Option<i32>,
    /// Cache read input tokens (Claude/Gemini)
    pub cache_read_input_tokens: Option<i32>,
    /// Reasoning output tokens (o1/Gemini)
    pub reasoning_output_tokens: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPayload {
    /// Notification message text
    pub text: String,
    /// Optional severity level (e.g., "info", "warning", "error")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialization() {
        let event = AgentEvent {
            id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            parent_id: None,
            timestamp: Utc::now(),
            stream_id: StreamId::Main,
            payload: EventPayload::User(UserPayload {
                text: "Hello".to_string(),
            }),
            metadata: None,
        };

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: AgentEvent = serde_json::from_str(&json).unwrap();

        match deserialized.payload {
            EventPayload::User(payload) => assert_eq!(payload.text, "Hello"),
            _ => panic!("Wrong payload type"),
        }
    }

    #[test]
    fn test_stream_id_variants() {
        // Test Main stream
        let main_stream = StreamId::Main;
        assert_eq!(main_stream.as_str(), "main");

        // Test Sidechain stream
        let sidechain_stream = StreamId::Sidechain {
            agent_id: "abc123".to_string(),
        };
        assert_eq!(sidechain_stream.as_str(), "sidechain:abc123");

        // Test Subagent stream
        let subagent_stream = StreamId::Subagent {
            name: "review".to_string(),
        };
        assert_eq!(subagent_stream.as_str(), "subagent:review");
    }
}
