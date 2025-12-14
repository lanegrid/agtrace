mod util;
pub mod v2;

use serde::{Deserialize, Serialize};

pub use util::*;

/// Git repository context
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GitContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_dirty: Option<bool>,
}

/// Execution environment context
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RunContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git: Option<GitContext>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime: Option<String>,
}

/// Agent control policy and constraints
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentPolicy {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sandbox_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_access: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval_policy: Option<String>,
}

/// Source of the agent log (provider-agnostic identifier)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Source(String);

impl Source {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
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

/// File operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileOp {
    Read,
    Write,
    Modify,
    Delete,
    Create,
    Move,
}

/// Normalized tool name (standardized across providers)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ToolName {
    /// Shell command execution (bash, sh, zsh)
    Bash,
    /// File read operation
    Read,
    /// File write operation
    Write,
    /// File edit operation
    Edit,
    /// File pattern search (glob)
    Glob,
    /// Content search (grep)
    Grep,
    /// Other tools not in standard set
    Other(String),
}

impl ToolName {
    /// Convert to string representation
    pub fn as_str(&self) -> &str {
        match self {
            ToolName::Bash => "Bash",
            ToolName::Read => "Read",
            ToolName::Write => "Write",
            ToolName::Edit => "Edit",
            ToolName::Glob => "Glob",
            ToolName::Grep => "Grep",
            ToolName::Other(s) => s.as_str(),
        }
    }

    /// Get the channel for this tool
    pub fn channel(&self) -> Channel {
        match self {
            ToolName::Bash => Channel::Terminal,
            ToolName::Read | ToolName::Write | ToolName::Edit => Channel::Editor,
            ToolName::Glob | ToolName::Grep => Channel::Filesystem,
            ToolName::Other(_) => Channel::Chat,
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
