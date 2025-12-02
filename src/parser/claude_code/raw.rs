use serde::{Deserialize, Serialize};

/// Claude Code JSONL message format (as-is model capturing all fields)
#[cfg_attr(test, derive(Serialize))]
#[cfg_attr(test, serde(deny_unknown_fields))]
#[derive(Debug, Deserialize)]
pub struct ClaudeCodeMessage {
    #[serde(rename = "type")]
    pub(crate) msg_type: String,
    #[serde(rename = "sessionId")]
    pub(crate) session_id: Option<String>,
    #[serde(rename = "messageId")]
    pub(crate) message_id: Option<String>,
    pub(crate) timestamp: Option<String>,
    pub(crate) cwd: Option<String>,
    #[serde(rename = "gitBranch")]
    pub(crate) git_branch: Option<String>,
    pub(crate) message: Option<MessageContent>,
    pub(crate) text: Option<String>,
    pub(crate) snapshot: Option<SnapshotInfo>,
    // Additional fields from actual format
    #[serde(rename = "agentId")]
    pub(crate) _agent_id: Option<String>,
    pub(crate) _uuid: Option<String>,
    #[serde(rename = "parentUuid")]
    pub(crate) _parent_uuid: Option<String>,
    #[serde(rename = "isSidechain")]
    pub(crate) _is_sidechain: Option<bool>,
    #[serde(rename = "userType")]
    pub(crate) _user_type: Option<String>,
    #[serde(rename = "isMeta")]
    pub(crate) _is_meta: Option<bool>,
    #[serde(rename = "thinkingMetadata")]
    pub(crate) _thinking_metadata: Option<serde_json::Value>,
    pub(crate) version: Option<String>,
}

#[cfg_attr(test, derive(Serialize))]
#[cfg_attr(test, serde(deny_unknown_fields))]
#[derive(Debug, Deserialize)]
pub(crate) struct SnapshotInfo {
    #[serde(rename = "messageId")]
    pub(crate) message_id: Option<String>,
    pub(crate) timestamp: Option<String>,
    #[serde(rename = "trackedFileBackups")]
    pub(crate) _tracked_file_backups: Option<serde_json::Value>,
}

#[cfg_attr(test, derive(Serialize))]
#[cfg_attr(test, serde(deny_unknown_fields))]
#[derive(Debug, Deserialize)]
pub(crate) struct MessageContent {
    pub(crate) role: Option<String>,
    #[serde(deserialize_with = "deserialize_content")]
    pub(crate) content: Option<Vec<ContentBlock>>,
    pub(crate) usage: Option<Usage>,
    pub(crate) model: Option<String>,
}

fn deserialize_content<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<Vec<ContentBlock>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Deserialize;

    let value = Option::<serde_json::Value>::deserialize(deserializer)?;

    match value {
        None => Ok(None),
        Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(s)) => {
            // Convert string to a text content block
            Ok(Some(vec![ContentBlock::Text { text: s }]))
        }
        Some(serde_json::Value::Array(arr)) => {
            // Try to deserialize as Vec<ContentBlock>
            let blocks: Vec<ContentBlock> = serde_json::from_value(serde_json::Value::Array(arr))
                .map_err(serde::de::Error::custom)?;
            Ok(Some(blocks))
        }
        Some(_) => {
            // For any other type (object, number, bool), return None or an empty vec
            Ok(None)
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
pub(crate) enum ContentBlock {
    #[serde(rename = "text", alias = "input_text")]
    Text { text: String },
    #[serde(rename = "thinking")]
    Thinking { thinking: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        name: String,
        input: serde_json::Value,
        id: Option<String>,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: Option<String>,
        #[serde(default)]
        is_error: Option<bool>,
        #[serde(deserialize_with = "deserialize_tool_result_content")]
        content: Option<Vec<ToolResultContent>>,
    },
}

fn deserialize_tool_result_content<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<Vec<ToolResultContent>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Deserialize;

    let value = Option::<serde_json::Value>::deserialize(deserializer)?;

    match value {
        None => Ok(None),
        Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(s)) => {
            // Convert string to a ToolResultContent::String
            Ok(Some(vec![ToolResultContent::String(s)]))
        }
        Some(serde_json::Value::Array(arr)) => {
            // Try to deserialize as Vec<ToolResultContent>
            let contents: Vec<ToolResultContent> =
                serde_json::from_value(serde_json::Value::Array(arr))
                    .map_err(serde::de::Error::custom)?;
            Ok(Some(contents))
        }
        Some(_) => {
            // For any other type, return None
            Ok(None)
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub(crate) enum ToolResultContent {
    Text { text: String },
    String(String),
    // Catch-all for any other JSON value (objects, arrays, numbers, bools, null)
    Other(serde_json::Value),
}

#[cfg_attr(test, derive(Serialize))]
#[derive(Debug, Deserialize)]
pub(crate) struct Usage {
    pub(crate) input_tokens: Option<u64>,
    pub(crate) output_tokens: Option<u64>,
    pub(crate) cache_read_input_tokens: Option<u64>,
    pub(crate) cache_creation_input_tokens: Option<u64>,
}
