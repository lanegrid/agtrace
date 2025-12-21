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

/// Normalized tool call with structured arguments
///
/// This enum provides type-safe access to common tool call patterns while
/// maintaining compatibility with the original JSON structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolCallPayload {
    /// File read operation (Read, Glob, etc.)
    FileRead {
        name: String,
        arguments: FileReadArgs,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        provider_call_id: Option<String>,
    },

    /// File edit operation (Edit)
    FileEdit {
        name: String,
        arguments: FileEditArgs,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        provider_call_id: Option<String>,
    },

    /// File write operation (Write)
    FileWrite {
        name: String,
        arguments: FileWriteArgs,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        provider_call_id: Option<String>,
    },

    /// Execute/shell command (Bash, etc.)
    Execute {
        name: String,
        arguments: ExecuteArgs,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        provider_call_id: Option<String>,
    },

    /// Search operation (Grep, WebSearch, etc.)
    Search {
        name: String,
        arguments: SearchArgs,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        provider_call_id: Option<String>,
    },

    /// MCP (Model Context Protocol) tool call
    Mcp {
        name: String,
        arguments: McpArgs,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        provider_call_id: Option<String>,
    },

    /// Generic/fallback for unknown or custom tools
    Generic {
        name: String,
        arguments: Value,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        provider_call_id: Option<String>,
    },
}

impl ToolCallPayload {
    /// Get tool name regardless of variant
    pub fn name(&self) -> &str {
        match self {
            ToolCallPayload::FileRead { name, .. } => name,
            ToolCallPayload::FileEdit { name, .. } => name,
            ToolCallPayload::FileWrite { name, .. } => name,
            ToolCallPayload::Execute { name, .. } => name,
            ToolCallPayload::Search { name, .. } => name,
            ToolCallPayload::Mcp { name, .. } => name,
            ToolCallPayload::Generic { name, .. } => name,
        }
    }

    /// Get provider call ID regardless of variant
    pub fn provider_call_id(&self) -> Option<&str> {
        match self {
            ToolCallPayload::FileRead { provider_call_id, .. } => provider_call_id.as_deref(),
            ToolCallPayload::FileEdit { provider_call_id, .. } => provider_call_id.as_deref(),
            ToolCallPayload::FileWrite { provider_call_id, .. } => provider_call_id.as_deref(),
            ToolCallPayload::Execute { provider_call_id, .. } => provider_call_id.as_deref(),
            ToolCallPayload::Search { provider_call_id, .. } => provider_call_id.as_deref(),
            ToolCallPayload::Mcp { provider_call_id, .. } => provider_call_id.as_deref(),
            ToolCallPayload::Generic { provider_call_id, .. } => provider_call_id.as_deref(),
        }
    }

    /// Create a normalized ToolCallPayload from raw name and arguments
    pub fn from_raw(name: String, arguments: Value, provider_call_id: Option<String>) -> Self {
        // Try to parse into specific variants based on name
        match name.as_str() {
            "Read" | "Glob" => {
                if let Ok(args) = serde_json::from_value(arguments.clone()) {
                    return ToolCallPayload::FileRead { name, arguments: args, provider_call_id };
                }
            }
            "Edit" => {
                if let Ok(args) = serde_json::from_value(arguments.clone()) {
                    return ToolCallPayload::FileEdit { name, arguments: args, provider_call_id };
                }
            }
            "Write" => {
                if let Ok(args) = serde_json::from_value(arguments.clone()) {
                    return ToolCallPayload::FileWrite { name, arguments: args, provider_call_id };
                }
            }
            "Bash" | "KillShell" | "BashOutput" => {
                if let Ok(args) = serde_json::from_value(arguments.clone()) {
                    return ToolCallPayload::Execute { name, arguments: args, provider_call_id };
                }
            }
            "Grep" | "WebSearch" | "WebFetch" => {
                if let Ok(args) = serde_json::from_value(arguments.clone()) {
                    return ToolCallPayload::Search { name, arguments: args, provider_call_id };
                }
            }
            _ if name.starts_with("mcp__") => {
                if let Ok(args) = serde_json::from_value(arguments.clone()) {
                    return ToolCallPayload::Mcp { name, arguments: args, provider_call_id };
                }
            }
            _ => {}
        }

        // Fallback to generic
        ToolCallPayload::Generic { name, arguments, provider_call_id }
    }
}

// --- Tool Arguments ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileReadArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    #[serde(flatten)]
    pub extra: Value,
}

impl FileReadArgs {
    /// Get file path from various field names
    pub fn path(&self) -> Option<&str> {
        self.file_path.as_deref().or(self.path.as_deref())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEditArgs {
    pub file_path: String,
    pub old_string: String,
    pub new_string: String,
    #[serde(default)]
    pub replace_all: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileWriteArgs {
    pub file_path: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
    #[serde(flatten)]
    pub extra: Value,
}

impl ExecuteArgs {
    pub fn command(&self) -> Option<&str> {
        self.command.as_deref()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(flatten)]
    pub extra: Value,
}

impl SearchArgs {
    /// Get search pattern from various field names
    pub fn pattern(&self) -> Option<&str> {
        self.pattern.as_deref()
            .or(self.query.as_deref())
            .or(self.input.as_deref())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpArgs {
    #[serde(flatten)]
    pub inner: Value,
}

impl McpArgs {
    /// Parse MCP tool name from full name (e.g., "mcp__o3__o3-search" -> ("o3", "o3-search"))
    pub fn parse_name(full_name: &str) -> Option<(String, String)> {
        if !full_name.starts_with("mcp__") {
            return None;
        }

        let rest = &full_name[5..]; // Remove "mcp__"
        let parts: Vec<&str> = rest.splitn(2, "__").collect();

        if parts.len() == 2 {
            Some((parts[0].to_string(), parts[1].to_string()))
        } else {
            None
        }
    }

    /// Get server name from full MCP tool name
    pub fn server_name(full_name: &str) -> Option<String> {
        Self::parse_name(full_name).map(|(server, _)| server)
    }

    /// Get tool name from full MCP tool name
    pub fn tool_name(full_name: &str) -> Option<String> {
        Self::parse_name(full_name).map(|(_, tool)| tool)
    }
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
