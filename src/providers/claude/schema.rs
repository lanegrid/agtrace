use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "kebab-case")]
pub enum ClaudeRecord {
    FileHistorySnapshot(FileHistorySnapshotRecord),
    User(UserRecord),
    Assistant(AssistantRecord),
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FileHistorySnapshotRecord {
    pub message_id: String,
    pub snapshot: FileHistorySnapshot,
    #[serde(default)]
    pub is_snapshot_update: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FileHistorySnapshot {
    #[serde(default)]
    pub files: Vec<Value>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserRecord {
    pub uuid: String,
    pub parent_uuid: Option<String>,
    pub session_id: String,
    pub timestamp: String,
    pub message: UserMessage,
    #[serde(default)]
    pub is_sidechain: bool,
    #[serde(default)]
    pub is_meta: bool,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub git_branch: Option<String>,
    #[serde(default)]
    pub user_type: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub thinking_metadata: Option<Value>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UserMessage {
    pub role: String,
    #[serde(deserialize_with = "deserialize_user_content")]
    pub content: Vec<UserContent>,
}

fn deserialize_user_content<'de, D>(deserializer: D) -> Result<Vec<UserContent>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrArray {
        String(String),
        Array(Vec<UserContent>),
    }

    match StringOrArray::deserialize(deserializer)? {
        StringOrArray::String(s) => Ok(vec![UserContent::Text { text: s }]),
        StringOrArray::Array(arr) => Ok(arr),
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum UserContent {
    Text { text: String },
    Image { source: Value },
    ToolResult {
        tool_use_id: String,
        content: Value,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AssistantRecord {
    pub uuid: String,
    pub parent_uuid: Option<String>,
    pub session_id: String,
    pub timestamp: String,
    pub message: AssistantMessage,
    #[serde(default)]
    pub is_sidechain: bool,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub git_branch: Option<String>,
    #[serde(default)]
    pub user_type: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub request_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AssistantMessage {
    #[serde(rename = "type")]
    pub message_type: String,
    pub id: String,
    pub role: String,
    pub model: String,
    pub content: Vec<AssistantContent>,
    #[serde(default)]
    pub stop_reason: Option<String>,
    #[serde(default)]
    pub stop_sequence: Option<String>,
    #[serde(default)]
    pub usage: Option<TokenUsage>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum AssistantContent {
    Text {
        text: String,
        #[serde(default)]
        signature: Option<Value>,
    },
    Thinking {
        thinking: String,
        #[serde(default)]
        signature: Option<Value>,
    },
    ToolUse {
        id: String,
        name: String,
        input: Value,
        #[serde(default)]
        signature: Option<Value>,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(default)]
        is_error: bool,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    #[serde(default)]
    pub cache_creation_input_tokens: Option<u32>,
    #[serde(default)]
    pub cache_read_input_tokens: Option<u32>,
}
