use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum CodexRecord {
    SessionMeta(SessionMetaRecord),
    ResponseItem(ResponseItemRecord),
    EventMsg(EventMsgRecord),
    TurnContext(TurnContextRecord),
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SessionMetaRecord {
    pub timestamp: String,
    pub payload: SessionMetaPayload,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SessionMetaPayload {
    pub id: String,
    pub timestamp: String,
    pub cwd: String,
    pub originator: String,
    pub cli_version: String,
    #[serde(default)]
    pub instructions: Option<String>,
    pub source: String,
    pub model_provider: String,
    #[serde(default)]
    pub git: Option<GitInfo>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GitInfo {
    pub commit_hash: String,
    pub branch: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ResponseItemRecord {
    pub timestamp: String,
    pub payload: ResponseItemPayload,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum ResponseItemPayload {
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
pub struct MessagePayload {
    pub role: String,
    pub content: Vec<MessageContent>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum MessageContent {
    InputText { text: String },
    OutputText { text: String },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ReasoningPayload {
    pub summary: Vec<SummaryText>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub encrypted_content: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum SummaryText {
    SummaryText { text: String },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FunctionCallPayload {
    pub name: String,
    pub arguments: String,
    pub call_id: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FunctionCallOutputPayload {
    pub call_id: String,
    pub output: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CustomToolCallPayload {
    pub status: String,
    pub call_id: String,
    pub name: String,
    pub input: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CustomToolCallOutputPayload {
    pub call_id: String,
    pub output: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GhostSnapshotPayload {
    pub ghost_commit: GhostCommit,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GhostCommit {
    pub id: String,
    pub parent: String,
    #[serde(default)]
    pub preexisting_untracked_files: Vec<String>,
    #[serde(default)]
    pub preexisting_untracked_dirs: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EventMsgRecord {
    pub timestamp: String,
    pub payload: EventMsgPayload,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum EventMsgPayload {
    UserMessage(UserMessagePayload),
    AgentMessage(AgentMessagePayload),
    AgentReasoning(AgentReasoningPayload),
    TokenCount(TokenCountPayload),
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UserMessagePayload {
    pub message: String,
    #[serde(default)]
    pub images: Vec<Value>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AgentMessagePayload {
    pub message: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AgentReasoningPayload {
    pub text: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TokenCountPayload {
    #[serde(default)]
    pub info: Option<TokenInfo>,
    #[serde(default)]
    pub rate_limits: Option<Value>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TokenInfo {
    pub total_token_usage: TokenUsage,
    pub last_token_usage: TokenUsage,
    pub model_context_window: u32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TokenUsage {
    pub input_tokens: u32,
    #[serde(default)]
    pub cached_input_tokens: u32,
    pub output_tokens: u32,
    #[serde(default)]
    pub reasoning_output_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TurnContextRecord {
    pub timestamp: String,
    pub payload: TurnContextPayload,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TurnContextPayload {
    pub cwd: String,
    pub approval_policy: String,
    pub sandbox_policy: SandboxPolicy,
    pub model: String,
    pub effort: String,
    pub summary: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SandboxPolicy {
    #[serde(rename = "type")]
    pub policy_type: String,
    pub network_access: bool,
    #[serde(default)]
    pub exclude_tmpdir_env_var: bool,
    #[serde(default)]
    pub exclude_slash_tmp: bool,
}
