use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Source of the session (CLI or subagent)
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub(crate) enum SessionSource {
    /// Subagent session with type (e.g., {"subagent":"review"})
    Subagent { subagent: String },
    /// Regular CLI session (e.g., "cli")
    Cli(String),
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub(crate) enum CodexRecord {
    SessionMeta(SessionMetaRecord),
    ResponseItem(ResponseItemRecord),
    EventMsg(EventMsgRecord),
    TurnContext(TurnContextRecord),
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct SessionMetaRecord {
    pub timestamp: String,
    pub payload: SessionMetaPayload,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct SessionMetaPayload {
    pub id: String,
    pub timestamp: String,
    pub cwd: String,
    pub originator: String,
    pub cli_version: String,
    #[serde(default)]
    pub instructions: Option<String>,
    pub source: SessionSource,
    #[serde(default)]
    pub model_provider: Option<String>,
    #[serde(default)]
    pub git: Option<GitInfo>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct GitInfo {
    #[serde(default)]
    pub commit_hash: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub repository_url: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct ResponseItemRecord {
    pub timestamp: String,
    pub payload: ResponseItemPayload,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub(crate) enum ResponseItemPayload {
    Message(MessagePayload),
    Reasoning(ReasoningPayload),
    FunctionCall(FunctionCallPayload),
    FunctionCallOutput(FunctionCallOutputPayload),
    CustomToolCall(CustomToolCallPayload),
    CustomToolCallOutput(CustomToolCallOutputPayload),
    GhostSnapshot(GhostSnapshotPayload),
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct MessagePayload {
    pub role: String,
    pub content: Vec<MessageContent>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub(crate) enum MessageContent {
    InputText {
        text: String,
    },
    OutputText {
        text: String,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct ReasoningPayload {
    pub summary: Vec<SummaryText>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub encrypted_content: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub(crate) enum SummaryText {
    SummaryText {
        text: String,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct FunctionCallPayload {
    pub name: String,
    pub arguments: String,
    pub call_id: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct FunctionCallOutputPayload {
    pub call_id: String,
    pub output: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct CustomToolCallPayload {
    pub status: String,
    pub call_id: String,
    pub name: String,
    pub input: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct CustomToolCallOutputPayload {
    pub call_id: String,
    pub output: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct GhostSnapshotPayload {
    pub ghost_commit: GhostCommit,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct GhostCommit {
    pub id: String,
    pub parent: String,
    #[serde(default)]
    pub preexisting_untracked_files: Vec<String>,
    #[serde(default)]
    pub preexisting_untracked_dirs: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct EventMsgRecord {
    pub timestamp: String,
    pub payload: EventMsgPayload,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub(crate) enum EventMsgPayload {
    UserMessage(UserMessagePayload),
    AgentMessage(AgentMessagePayload),
    AgentReasoning(AgentReasoningPayload),
    TokenCount(TokenCountPayload),
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct UserMessagePayload {
    pub message: String,
    #[serde(default)]
    pub images: Vec<Value>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct AgentMessagePayload {
    pub message: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct AgentReasoningPayload {
    pub text: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct TokenCountPayload {
    #[serde(default)]
    pub info: Option<TokenInfo>,
    #[serde(default)]
    pub rate_limits: Option<Value>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct TokenInfo {
    pub total_token_usage: TokenUsage,
    pub last_token_usage: TokenUsage,
    pub model_context_window: u32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct TokenUsage {
    pub input_tokens: u32,
    #[serde(default)]
    pub cached_input_tokens: u32,
    pub output_tokens: u32,
    #[serde(default)]
    pub reasoning_output_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct TurnContextRecord {
    pub timestamp: String,
    pub payload: TurnContextPayload,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct TurnContextPayload {
    pub cwd: String,
    pub approval_policy: String,
    pub sandbox_policy: SandboxPolicy,
    pub model: String,
    #[serde(default)]
    pub effort: Option<String>,
    pub summary: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub(crate) enum SandboxPolicy {
    // New format (v0.63+): {"type": "read-only"}
    Simple {
        #[serde(rename = "type")]
        policy_type: String,
    },
    // Old format (v0.53): {"mode": "workspace-write", "network_access": false, ...}
    Detailed {
        mode: String,
        #[serde(default)]
        network_access: Option<bool>,
        #[serde(default)]
        exclude_tmpdir_env_var: bool,
        #[serde(default)]
        exclude_slash_tmp: bool,
    },
}
