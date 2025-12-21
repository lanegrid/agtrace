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
            ToolCallPayload::FileRead {
                provider_call_id, ..
            } => provider_call_id.as_deref(),
            ToolCallPayload::FileEdit {
                provider_call_id, ..
            } => provider_call_id.as_deref(),
            ToolCallPayload::FileWrite {
                provider_call_id, ..
            } => provider_call_id.as_deref(),
            ToolCallPayload::Execute {
                provider_call_id, ..
            } => provider_call_id.as_deref(),
            ToolCallPayload::Search {
                provider_call_id, ..
            } => provider_call_id.as_deref(),
            ToolCallPayload::Mcp {
                provider_call_id, ..
            } => provider_call_id.as_deref(),
            ToolCallPayload::Generic {
                provider_call_id, ..
            } => provider_call_id.as_deref(),
        }
    }

    /// Derive semantic ToolKind from ToolCallPayload variant
    pub fn kind(&self) -> ToolKind {
        match self {
            ToolCallPayload::FileRead { .. } => ToolKind::Read,
            ToolCallPayload::FileEdit { .. } => ToolKind::Write,
            ToolCallPayload::FileWrite { .. } => ToolKind::Write,
            ToolCallPayload::Execute { .. } => ToolKind::Execute,
            ToolCallPayload::Search { .. } => ToolKind::Search,
            ToolCallPayload::Mcp { .. } => ToolKind::Other,
            ToolCallPayload::Generic { .. } => ToolKind::Other,
        }
    }

    /// Create a normalized ToolCallPayload from raw name and arguments
    pub fn from_raw(name: String, arguments: Value, provider_call_id: Option<String>) -> Self {
        // Try to parse into specific variants based on name
        match name.as_str() {
            "Read" | "Glob" => {
                if let Ok(args) = serde_json::from_value(arguments.clone()) {
                    return ToolCallPayload::FileRead {
                        name,
                        arguments: args,
                        provider_call_id,
                    };
                }
            }
            "Edit" => {
                if let Ok(args) = serde_json::from_value(arguments.clone()) {
                    return ToolCallPayload::FileEdit {
                        name,
                        arguments: args,
                        provider_call_id,
                    };
                }
            }
            "Write" => {
                if let Ok(args) = serde_json::from_value(arguments.clone()) {
                    return ToolCallPayload::FileWrite {
                        name,
                        arguments: args,
                        provider_call_id,
                    };
                }
            }
            "Bash" | "KillShell" | "BashOutput" => {
                if let Ok(args) = serde_json::from_value(arguments.clone()) {
                    return ToolCallPayload::Execute {
                        name,
                        arguments: args,
                        provider_call_id,
                    };
                }
            }
            "Grep" | "WebSearch" | "WebFetch" => {
                if let Ok(args) = serde_json::from_value(arguments.clone()) {
                    return ToolCallPayload::Search {
                        name,
                        arguments: args,
                        provider_call_id,
                    };
                }
            }
            _ if name.starts_with("mcp__") => {
                if let Ok(args) = serde_json::from_value(arguments.clone()) {
                    return ToolCallPayload::Mcp {
                        name,
                        arguments: args,
                        provider_call_id,
                    };
                }
            }
            _ => {}
        }

        // Fallback to generic
        ToolCallPayload::Generic {
            name,
            arguments,
            provider_call_id,
        }
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
        self.pattern
            .as_deref()
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

    #[test]
    fn test_tool_call_from_raw_file_read() {
        let args = serde_json::json!({
            "file_path": "/path/to/file.rs"
        });
        let payload =
            ToolCallPayload::from_raw("Read".to_string(), args, Some("call_123".to_string()));

        assert_eq!(payload.name(), "Read");
        assert_eq!(payload.provider_call_id(), Some("call_123"));

        match payload {
            ToolCallPayload::FileRead {
                name,
                arguments,
                provider_call_id,
            } => {
                assert_eq!(name, "Read");
                assert_eq!(arguments.file_path, Some("/path/to/file.rs".to_string()));
                assert_eq!(provider_call_id, Some("call_123".to_string()));
            }
            _ => panic!("Expected FileRead variant"),
        }
    }

    #[test]
    fn test_tool_call_from_raw_glob() {
        let args = serde_json::json!({
            "pattern": "**/*.rs"
        });
        let payload = ToolCallPayload::from_raw("Glob".to_string(), args, None);

        match payload {
            ToolCallPayload::FileRead {
                name,
                arguments,
                provider_call_id,
            } => {
                assert_eq!(name, "Glob");
                assert_eq!(arguments.pattern, Some("**/*.rs".to_string()));
                assert_eq!(provider_call_id, None);
            }
            _ => panic!("Expected FileRead variant"),
        }
    }

    #[test]
    fn test_tool_call_from_raw_file_edit() {
        let args = serde_json::json!({
            "file_path": "/path/to/file.rs",
            "old_string": "old",
            "new_string": "new",
            "replace_all": true
        });
        let payload = ToolCallPayload::from_raw("Edit".to_string(), args, None);

        match payload {
            ToolCallPayload::FileEdit {
                name, arguments, ..
            } => {
                assert_eq!(name, "Edit");
                assert_eq!(arguments.file_path, "/path/to/file.rs");
                assert_eq!(arguments.old_string, "old");
                assert_eq!(arguments.new_string, "new");
                assert!(arguments.replace_all);
            }
            _ => panic!("Expected FileEdit variant"),
        }
    }

    #[test]
    fn test_tool_call_from_raw_file_write() {
        let args = serde_json::json!({
            "file_path": "/path/to/file.rs",
            "content": "fn main() {}"
        });
        let payload = ToolCallPayload::from_raw("Write".to_string(), args, None);

        match payload {
            ToolCallPayload::FileWrite {
                name, arguments, ..
            } => {
                assert_eq!(name, "Write");
                assert_eq!(arguments.file_path, "/path/to/file.rs");
                assert_eq!(arguments.content, "fn main() {}");
            }
            _ => panic!("Expected FileWrite variant"),
        }
    }

    #[test]
    fn test_tool_call_from_raw_execute() {
        let args = serde_json::json!({
            "command": "cargo test",
            "description": "Run tests",
            "timeout": 30000
        });
        let payload = ToolCallPayload::from_raw("Bash".to_string(), args, None);

        match payload {
            ToolCallPayload::Execute {
                name, arguments, ..
            } => {
                assert_eq!(name, "Bash");
                assert_eq!(arguments.command(), Some("cargo test"));
                assert_eq!(arguments.description, Some("Run tests".to_string()));
                assert_eq!(arguments.timeout, Some(30000));
            }
            _ => panic!("Expected Execute variant"),
        }
    }

    #[test]
    fn test_tool_call_from_raw_search() {
        let args = serde_json::json!({
            "pattern": "ToolCallPayload",
            "path": "src/"
        });
        let payload = ToolCallPayload::from_raw("Grep".to_string(), args, None);

        match payload {
            ToolCallPayload::Search {
                name, arguments, ..
            } => {
                assert_eq!(name, "Grep");
                assert_eq!(arguments.pattern(), Some("ToolCallPayload"));
                assert_eq!(arguments.path, Some("src/".to_string()));
            }
            _ => panic!("Expected Search variant"),
        }
    }

    #[test]
    fn test_tool_call_from_raw_mcp() {
        let args = serde_json::json!({
            "input": "search query"
        });
        let payload = ToolCallPayload::from_raw("mcp__o3__o3-search".to_string(), args, None);

        match payload {
            ToolCallPayload::Mcp {
                name, arguments, ..
            } => {
                assert_eq!(name, "mcp__o3__o3-search");
                assert_eq!(arguments.inner["input"], "search query");
            }
            _ => panic!("Expected Mcp variant"),
        }
    }

    #[test]
    fn test_tool_call_from_raw_generic_fallback() {
        let args = serde_json::json!({
            "custom_field": "value"
        });
        let payload = ToolCallPayload::from_raw("CustomTool".to_string(), args.clone(), None);

        match payload {
            ToolCallPayload::Generic {
                name, arguments, ..
            } => {
                assert_eq!(name, "CustomTool");
                assert_eq!(arguments, args);
            }
            _ => panic!("Expected Generic variant"),
        }
    }

    #[test]
    fn test_tool_call_serialization_roundtrip() {
        let original = ToolCallPayload::FileRead {
            name: "Read".to_string(),
            arguments: FileReadArgs {
                file_path: Some("/path/to/file.rs".to_string()),
                path: None,
                pattern: None,
                extra: serde_json::json!({}),
            },
            provider_call_id: Some("call_123".to_string()),
        };

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: ToolCallPayload = serde_json::from_str(&json).unwrap();

        match deserialized {
            ToolCallPayload::FileRead {
                name,
                arguments,
                provider_call_id,
            } => {
                assert_eq!(name, "Read");
                assert_eq!(arguments.file_path, Some("/path/to/file.rs".to_string()));
                assert_eq!(provider_call_id, Some("call_123".to_string()));
            }
            _ => panic!("Expected FileRead variant"),
        }
    }

    #[test]
    fn test_file_read_args_path_helper() {
        let args1 = FileReadArgs {
            file_path: Some("/path1".to_string()),
            path: None,
            pattern: None,
            extra: serde_json::json!({}),
        };
        assert_eq!(args1.path(), Some("/path1"));

        let args2 = FileReadArgs {
            file_path: None,
            path: Some("/path2".to_string()),
            pattern: None,
            extra: serde_json::json!({}),
        };
        assert_eq!(args2.path(), Some("/path2"));

        let args3 = FileReadArgs {
            file_path: Some("/path1".to_string()),
            path: Some("/path2".to_string()),
            pattern: None,
            extra: serde_json::json!({}),
        };
        assert_eq!(args3.path(), Some("/path1"));
    }

    #[test]
    fn test_search_args_pattern_helper() {
        let args1 = SearchArgs {
            pattern: Some("pattern1".to_string()),
            query: None,
            input: None,
            path: None,
            extra: serde_json::json!({}),
        };
        assert_eq!(args1.pattern(), Some("pattern1"));

        let args2 = SearchArgs {
            pattern: None,
            query: Some("query2".to_string()),
            input: None,
            path: None,
            extra: serde_json::json!({}),
        };
        assert_eq!(args2.pattern(), Some("query2"));

        let args3 = SearchArgs {
            pattern: None,
            query: None,
            input: Some("input3".to_string()),
            path: None,
            extra: serde_json::json!({}),
        };
        assert_eq!(args3.pattern(), Some("input3"));
    }

    #[test]
    fn test_mcp_args_parse_name() {
        assert_eq!(
            McpArgs::parse_name("mcp__o3__o3-search"),
            Some(("o3".to_string(), "o3-search".to_string()))
        );

        assert_eq!(
            McpArgs::parse_name("mcp__sqlite__query"),
            Some(("sqlite".to_string(), "query".to_string()))
        );

        assert_eq!(McpArgs::parse_name("not_mcp_tool"), None);
        assert_eq!(McpArgs::parse_name("mcp__only_server"), None);
    }

    #[test]
    fn test_mcp_args_server_and_tool_name() {
        assert_eq!(
            McpArgs::server_name("mcp__o3__o3-search"),
            Some("o3".to_string())
        );
        assert_eq!(
            McpArgs::tool_name("mcp__o3__o3-search"),
            Some("o3-search".to_string())
        );
    }

    #[test]
    fn test_tool_call_kind_derivation() {
        let read_payload = ToolCallPayload::FileRead {
            name: "Read".to_string(),
            arguments: FileReadArgs {
                file_path: Some("/path".to_string()),
                path: None,
                pattern: None,
                extra: serde_json::json!({}),
            },
            provider_call_id: None,
        };
        assert_eq!(read_payload.kind(), ToolKind::Read);

        let edit_payload = ToolCallPayload::FileEdit {
            name: "Edit".to_string(),
            arguments: FileEditArgs {
                file_path: "/path".to_string(),
                old_string: "old".to_string(),
                new_string: "new".to_string(),
                replace_all: false,
            },
            provider_call_id: None,
        };
        assert_eq!(edit_payload.kind(), ToolKind::Write);

        let write_payload = ToolCallPayload::FileWrite {
            name: "Write".to_string(),
            arguments: FileWriteArgs {
                file_path: "/path".to_string(),
                content: "content".to_string(),
            },
            provider_call_id: None,
        };
        assert_eq!(write_payload.kind(), ToolKind::Write);

        let exec_payload = ToolCallPayload::Execute {
            name: "Bash".to_string(),
            arguments: ExecuteArgs {
                command: Some("ls".to_string()),
                description: None,
                timeout: None,
                extra: serde_json::json!({}),
            },
            provider_call_id: None,
        };
        assert_eq!(exec_payload.kind(), ToolKind::Execute);

        let search_payload = ToolCallPayload::Search {
            name: "Grep".to_string(),
            arguments: SearchArgs {
                pattern: Some("pattern".to_string()),
                query: None,
                input: None,
                path: None,
                extra: serde_json::json!({}),
            },
            provider_call_id: None,
        };
        assert_eq!(search_payload.kind(), ToolKind::Search);

        let mcp_payload = ToolCallPayload::Mcp {
            name: "mcp__o3__search".to_string(),
            arguments: McpArgs {
                inner: serde_json::json!({}),
            },
            provider_call_id: None,
        };
        assert_eq!(mcp_payload.kind(), ToolKind::Other);

        let generic_payload = ToolCallPayload::Generic {
            name: "CustomTool".to_string(),
            arguments: serde_json::json!({}),
            provider_call_id: None,
        };
        assert_eq!(generic_payload.kind(), ToolKind::Other);
    }
}
