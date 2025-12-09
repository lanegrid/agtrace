use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Source of the agent log
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    ClaudeCode,
    Codex,
    Gemini,
}

/// Type of agent event
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    UserMessage,
    AssistantMessage,
    SystemMessage,
    Reasoning,
    ToolCall,
    ToolResult,
    FileSnapshot,
    SessionSummary,
    Meta,
    Log,
}

/// Role of the event actor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    User,
    Assistant,
    System,
    Tool,
    Cli,
    Other,
}

/// Communication channel
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    Chat,
    Editor,
    Terminal,
    Filesystem,
    System,
    Other,
}

/// Tool execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolStatus {
    Success,
    Error,
    InProgress,
    Unknown,
}

/// Normalized agent event (v1)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEventV1 {
    pub schema_version: String,
    pub source: Source,

    // Project / Session / ID
    pub project_hash: String,
    pub project_root: Option<String>,
    pub session_id: Option<String>,
    pub event_id: Option<String>,
    pub parent_event_id: Option<String>,
    pub ts: String,

    // Event properties
    pub event_type: EventType,
    pub role: Option<Role>,
    pub channel: Option<Channel>,
    pub text: Option<String>,

    // Tool / Command
    pub tool_name: Option<String>,
    pub tool_call_id: Option<String>,
    pub tool_status: Option<ToolStatus>,
    pub tool_latency_ms: Option<u64>,
    pub tool_exit_code: Option<i32>,

    // File / Code
    pub file_path: Option<String>,
    pub file_language: Option<String>,
    pub file_op: Option<String>,

    // Model / Tokens
    pub model: Option<String>,
    pub tokens_input: Option<u64>,
    pub tokens_output: Option<u64>,
    pub tokens_total: Option<u64>,
    pub tokens_cached: Option<u64>,
    pub tokens_thinking: Option<u64>,
    pub tokens_tool: Option<u64>,

    pub agent_id: Option<String>,

    // Raw vendor data
    pub raw: serde_json::Value,
}

impl AgentEventV1 {
    pub const SCHEMA_VERSION: &'static str = "agtrace.event.v1";

    pub fn new(source: Source, project_hash: String, ts: String, event_type: EventType) -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION.to_string(),
            source,
            project_hash,
            project_root: None,
            session_id: None,
            event_id: None,
            parent_event_id: None,
            ts,

            event_type,
            role: None,
            channel: None,
            text: None,

            tool_name: None,
            tool_call_id: None,
            tool_status: None,
            tool_latency_ms: None,
            tool_exit_code: None,

            file_path: None,
            file_language: None,
            file_op: None,

            model: None,
            tokens_input: None,
            tokens_output: None,
            tokens_total: None,
            tokens_cached: None,
            tokens_thinking: None,
            tokens_tool: None,

            agent_id: None,
            raw: serde_json::Value::Null,
        }
    }
}

/// Session summary for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub session_id: String,
    pub source: Source,
    pub project_hash: String,
    pub start_ts: String,
    pub end_ts: String,
    pub event_count: usize,
    pub user_message_count: usize,
    pub tokens_input_total: u64,
    pub tokens_output_total: u64,
}

impl FromStr for Source {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "claude" => Ok(Source::ClaudeCode),
            "codex" => Ok(Source::Codex),
            "gemini" => Ok(Source::Gemini),
            _ => Err(anyhow!("Unknown source: {}", s)),
        }
    }
}

impl FromStr for EventType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "user_message" => Ok(EventType::UserMessage),
            "assistant_message" => Ok(EventType::AssistantMessage),
            "reasoning" => Ok(EventType::Reasoning),
            "tool_call" => Ok(EventType::ToolCall),
            "tool_result" => Ok(EventType::ToolResult),
            "meta" => Ok(EventType::Meta),
            _ => Err(anyhow!("Unknown event type: {}", s)),
        }
    }
}
