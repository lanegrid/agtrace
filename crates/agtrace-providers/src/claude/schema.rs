use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ClaudeRecord {
    FileHistorySnapshot(FileHistorySnapshotRecord),
    User(UserRecord),
    Assistant(AssistantRecord),
    System(SystemRecord),
    Progress(ProgressRecord),
    QueueOperation(QueueOperationRecord),
    Summary(SummaryRecord),
    PrLink(PrLinkRecord),
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FileHistorySnapshotRecord {
    pub message_id: String,
    pub snapshot: FileHistorySnapshot,
    #[serde(default)]
    pub is_snapshot_update: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct FileHistorySnapshot {
    #[serde(default)]
    pub files: Vec<Value>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UserRecord {
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
    pub agent_id: Option<String>,
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
    /// Subagent execution result (contains agentId for sidechain linking)
    #[serde(default, skip_serializing_if = "skip_empty_tool_use_result")]
    pub tool_use_result: Option<ToolUseResult>,
    /// Session slug (human-readable session name)
    #[serde(default)]
    pub slug: Option<String>,
    /// Whether this is a compaction summary message
    #[serde(default)]
    pub is_compact_summary: bool,
    /// Plan content text (from plan mode)
    #[serde(default)]
    pub plan_content: Option<String>,
}

/// Subagent execution result metadata
#[derive(Debug, Clone, Default)]
pub(crate) struct ToolUseResult {
    pub status: Option<String>,
    pub prompt: Option<String>,
    /// Agent ID linking this tool result to its sidechain (e.g., "be466c0a")
    pub agent_id: Option<String>,
}

impl ToolUseResult {
    /// Check if all fields are None (considered "empty" for serialization skip)
    pub fn is_empty(&self) -> bool {
        self.status.is_none() && self.prompt.is_none() && self.agent_id.is_none()
    }
}

/// Skip serializing Option<ToolUseResult> if None or empty
pub(crate) fn skip_empty_tool_use_result(opt: &Option<ToolUseResult>) -> bool {
    match opt {
        None => true,
        Some(r) => r.is_empty(),
    }
}

impl<'de> serde::Deserialize<'de> for ToolUseResult {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{MapAccess, Visitor};
        use std::fmt;

        struct ToolUseResultVisitor;

        impl<'de> Visitor<'de> for ToolUseResultVisitor {
            type Value = ToolUseResult;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a map or any value for ToolUseResult")
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut result = ToolUseResult::default();
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "status" => result.status = map.next_value()?,
                        "prompt" => result.prompt = map.next_value()?,
                        "agentId" => result.agent_id = map.next_value()?,
                        _ => {
                            let _ = map.next_value::<serde::de::IgnoredAny>()?;
                        }
                    }
                }
                Ok(result)
            }

            // Handle string values (e.g., error messages)
            fn visit_str<E>(self, _: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(ToolUseResult::default())
            }

            fn visit_string<E>(self, _: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(ToolUseResult::default())
            }

            // Handle array values (e.g., [{"type": "text", "text": "..."}])
            fn visit_seq<S>(self, mut seq: S) -> Result<Self::Value, S::Error>
            where
                S: serde::de::SeqAccess<'de>,
            {
                // Consume the sequence but ignore its contents
                while seq.next_element::<serde::de::IgnoredAny>()?.is_some() {}
                Ok(ToolUseResult::default())
            }
        }

        deserializer.deserialize_any(ToolUseResultVisitor)
    }
}

impl serde::Serialize for ToolUseResult {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;
        if let Some(ref status) = self.status {
            map.serialize_entry("status", status)?;
        }
        if let Some(ref prompt) = self.prompt {
            map.serialize_entry("prompt", prompt)?;
        }
        if let Some(ref agent_id) = self.agent_id {
            map.serialize_entry("agentId", agent_id)?;
        }
        map.end()
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct UserMessage {
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
pub(crate) enum UserContent {
    Text {
        text: String,
    },
    Image {
        source: Value,
    },
    ToolResult {
        tool_use_id: String,
        #[serde(default)]
        content: Option<Value>,
        #[serde(default)]
        is_error: bool,
        /// Agent ID for subagent execution (e.g., "ba2ed465")
        #[serde(default, rename = "agentId")]
        agent_id: Option<String>,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AssistantRecord {
    pub uuid: String,
    pub parent_uuid: Option<String>,
    pub session_id: String,
    pub timestamp: String,
    pub message: AssistantMessage,
    #[serde(default)]
    pub is_sidechain: bool,
    #[serde(default)]
    pub agent_id: Option<String>,
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
    /// Session slug (human-readable session name)
    #[serde(default)]
    pub slug: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct AssistantMessage {
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
pub(crate) enum AssistantContent {
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

/// System record (e.g., slash command execution logs, turn duration, compaction)
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SystemRecord {
    pub uuid: String,
    #[serde(default)]
    pub parent_uuid: Option<String>,
    pub session_id: String,
    pub timestamp: String,
    pub subtype: String,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub level: Option<String>,
    #[serde(default)]
    pub is_sidechain: bool,
    #[serde(default)]
    pub is_meta: bool,
    /// Turn duration in milliseconds (subtype: "turn_duration")
    #[serde(default)]
    pub duration_ms: Option<u64>,
    /// Compaction metadata (subtype: "compact_boundary")
    #[serde(default)]
    pub compact_metadata: Option<CompactMetadata>,
    /// Number of hooks executed (subtype: "stop_hook_summary")
    #[serde(default)]
    pub hook_count: Option<u32>,
    /// Hook execution details (subtype: "stop_hook_summary")
    #[serde(default)]
    pub hook_infos: Option<Vec<HookInfo>>,
    /// Whether hooks prevented continuation (subtype: "stop_hook_summary")
    #[serde(default)]
    pub prevented_continuation: bool,
}

/// Hook execution info for stop_hook_summary
#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct HookInfo {
    #[serde(default)]
    pub command: Option<String>,
}

/// Progress record (subagent progress, hook progress, etc.)
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProgressRecord {
    pub uuid: String,
    #[serde(default)]
    pub parent_uuid: Option<String>,
    pub session_id: String,
    pub timestamp: String,
    pub data: ProgressData,
    #[serde(default)]
    pub tool_use_id: Option<String>,
    #[serde(default)]
    pub parent_tool_use_id: Option<String>,
    #[serde(default)]
    pub is_sidechain: bool,
    #[serde(default)]
    pub agent_id: Option<String>,
}

/// Progress data variants
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub(crate) enum ProgressData {
    AgentProgress {
        #[serde(default, rename = "agentId")]
        agent_id: Option<String>,
        #[serde(default)]
        status: Option<String>,
        #[serde(default)]
        prompt: Option<String>,
    },
    HookProgress {
        #[serde(rename = "hookEvent")]
        hook_event: String,
        #[serde(default, rename = "hookName")]
        hook_name: Option<String>,
        #[serde(default)]
        command: Option<String>,
    },
    #[serde(other)]
    Other,
}

/// Queue operation record (background task queue)
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct QueueOperationRecord {
    pub operation: String,
    pub timestamp: String,
    pub session_id: String,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub task_id: Option<String>,
}

/// Summary record
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SummaryRecord {
    pub summary: String,
    #[serde(default)]
    pub leaf_uuid: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub timestamp: Option<String>,
}

/// Compaction metadata for compact_boundary system events
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CompactMetadata {
    #[serde(default)]
    pub trigger: Option<String>,
    #[serde(default)]
    pub pre_tokens: Option<u64>,
}

/// PR link record (associates a pull request with a session)
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PrLinkRecord {
    pub session_id: String,
    pub timestamp: String,
    pub pr_number: u64,
    pub pr_url: String,
    pub pr_repository: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    #[serde(default)]
    pub cache_creation_input_tokens: Option<u32>,
    #[serde(default)]
    pub cache_read_input_tokens: Option<u32>,
    /// Detailed cache creation breakdown by TTL tier
    #[serde(default)]
    pub cache_creation: Option<CacheCreationDetail>,
}

/// Detailed cache creation token breakdown by TTL tier
#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct CacheCreationDetail {
    #[serde(default)]
    pub ephemeral_5m_input_tokens: Option<u32>,
    #[serde(default)]
    pub ephemeral_1h_input_tokens: Option<u32>,
}
