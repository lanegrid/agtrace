//! Message content blocks of Claude Code `user` / `assistant` records.
//!
//! Everything not needed to identify a block is optional; unknown block types
//! deserialize to `Unknown` instead of failing the record.

use serde::Deserialize;
use serde_json::Value;

/// `message` of a `user` record. `content` is a string or an array of blocks.
#[derive(Debug, Deserialize, Clone, Default)]
pub(crate) struct UserMessage {
    #[serde(default, deserialize_with = "deserialize_user_content")]
    pub content: Vec<UserContent>,
}

fn deserialize_user_content<'de, D>(deserializer: D) -> Result<Vec<UserContent>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrArray {
        String(String),
        Array(Vec<UserContent>),
        Null(()),
    }

    Ok(match StringOrArray::deserialize(deserializer)? {
        StringOrArray::String(text) => vec![UserContent::Text { text }],
        StringOrArray::Array(blocks) => blocks,
        StringOrArray::Null(()) => Vec::new(),
    })
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum UserContent {
    Text {
        #[serde(default)]
        text: String,
    },
    ToolResult {
        tool_use_id: String,
        /// String or array of content items (text / image / tool_reference / document).
        #[serde(default)]
        content: Value,
        #[serde(default)]
        is_error: bool,
        /// Legacy: subagent id on the result block.
        #[serde(default, rename = "agentId")]
        agent_id: Option<String>,
    },
    #[serde(other)]
    Unknown,
}

/// `message` of an `assistant` record (one content block per record since 2.1.x).
#[derive(Debug, Deserialize, Clone, Default)]
pub(crate) struct AssistantMessage {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub content: Vec<AssistantContent>,
    #[serde(default)]
    pub stop_reason: Option<String>,
    #[serde(default)]
    pub usage: Option<Usage>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum AssistantContent {
    Text {
        #[serde(default)]
        text: String,
    },
    Thinking {
        #[serde(default)]
        thinking: String,
        #[serde(default)]
        signature: Option<Value>,
    },
    ToolUse {
        id: String,
        name: String,
        #[serde(default)]
        input: Value,
    },
    #[serde(other)]
    Unknown,
}

/// `message.usage`. Final write mode carries `iterations`; stream-start mode does not.
#[derive(Debug, Deserialize, Clone, Default)]
pub(crate) struct Usage {
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default)]
    pub cache_creation_input_tokens: Option<u64>,
    #[serde(default)]
    pub cache_read_input_tokens: Option<u64>,
    #[serde(default)]
    pub output_tokens_details: Option<OutputTokensDetails>,
    /// Present (non-null) only in the final write mode.
    #[serde(default)]
    pub iterations: Option<Value>,
}

/// `thinking_tokens` is already included in `output_tokens`.
#[derive(Debug, Deserialize, Clone, Default)]
pub(crate) struct OutputTokensDetails {
    #[serde(default)]
    pub thinking_tokens: Option<u64>,
}
