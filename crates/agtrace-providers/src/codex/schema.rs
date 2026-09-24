use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    #[serde(default)]
    pub timestamp: Option<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub originator: Option<String>,
    #[serde(default)]
    pub cli_version: Option<String>,
    #[serde(default)]
    pub instructions: Option<String>,
    /// `"cli"` / `"vscode"` / `{"subagent":"review"}` /
    /// `{"subagent":{"thread_spawn":{...}}}` — kept untyped, read leniently.
    #[serde(default)]
    pub source: Option<Value>,
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
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct MessagePayload {
    pub role: String,
    #[serde(default)]
    pub content: Vec<MessageContent>,
    /// Assistant message phase (e.g. "commentary", "final_answer")
    #[serde(default)]
    pub phase: Option<String>,
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
    #[serde(default)]
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
    /// String or content-item array (flattened)
    #[serde(deserialize_with = "crate::lenient::deserialize_flat_output")]
    pub output: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct CustomToolCallPayload {
    #[serde(default)]
    pub status: Option<String>,
    pub call_id: String,
    pub name: String,
    pub input: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct CustomToolCallOutputPayload {
    pub call_id: String,
    /// String or content-item array (flattened)
    #[serde(deserialize_with = "crate::lenient::deserialize_flat_output")]
    pub output: String,
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
    /// Event when entering review mode (spawns a subagent)
    EnteredReviewMode(EnteredReviewModePayload),
    #[serde(other)]
    Unknown,
}

/// Payload for entered_review_mode event (subagent spawn signal)
///
/// Codex has two formats:
/// - Old (cli_version < 0.77): { "prompt": "...", "user_facing_hint": "..." }
/// - New (cli_version >= 0.77): { "target": {...}, "user_facing_hint": "..." }
#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct EnteredReviewModePayload {
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub target: Option<serde_json::Value>,
    #[serde(default)]
    pub user_facing_hint: Option<String>,
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
    #[serde(default)]
    pub total_token_usage: Option<TokenUsage>,
    pub last_token_usage: TokenUsage,
    /// Nullable in recent Codex versions
    #[serde(default)]
    pub model_context_window: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub(crate) struct TokenUsage {
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub cached_input_tokens: u64,
    #[serde(default)]
    pub cache_write_input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default)]
    pub reasoning_output_tokens: u64,
    #[serde(default)]
    pub total_tokens: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct TurnContextRecord {
    pub timestamp: String,
    pub payload: TurnContextPayload,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct TurnContextPayload {
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub approval_policy: Option<Value>,
    #[serde(default)]
    pub sandbox_policy: Option<Value>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub effort: Option<String>,
    #[serde(default)]
    pub summary: Option<Value>,
}

/// Subagent label from a `session_meta.source` value, if the session is a child.
///
/// - `{"subagent":"review"}` (legacy) → `"review"`
/// - `{"subagent":{"thread_spawn":{...}}}` → `agent_role`, else `"thread_spawn"`
pub(crate) fn source_subagent_label(source: &Value) -> Option<String> {
    let sub = source.get("subagent")?;
    if let Some(s) = sub.as_str() {
        return Some(s.to_string());
    }
    let spawn = sub.get("thread_spawn");
    spawn
        .and_then(|t| t.get("agent_role"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| sub.as_object().and_then(|o| o.keys().next().cloned()))
}

#[cfg(test)]
mod source_tests {
    use super::*;

    #[test]
    fn legacy_subagent_string() {
        let v = serde_json::json!({"subagent":"review"});
        assert_eq!(source_subagent_label(&v).as_deref(), Some("review"));
    }

    #[test]
    fn thread_spawn_object() {
        let v = serde_json::json!({"subagent":{"thread_spawn":{"parent_thread_id":"p","depth":1,"agent_path":"/root/judge","agent_nickname":"Judge","agent_role":null}}});
        assert_eq!(source_subagent_label(&v).as_deref(), Some("thread_spawn"));
    }

    #[test]
    fn cli_source_is_root() {
        assert_eq!(source_subagent_label(&serde_json::json!("cli")), None);
    }

    #[test]
    fn session_meta_with_thread_spawn_source_parses() {
        let line = r#"{"timestamp":"2026-09-20T10:00:00Z","type":"session_meta","payload":{"id":"01900000-0000-7000-8000-000000000002","session_id":"01900000-0000-7000-8000-000000000001","timestamp":"2026-09-20T10:00:00Z","cwd":"/work/demo-project","originator":"codex_cli_rs","cli_version":"0.153.0","source":{"subagent":{"thread_spawn":{"parent_thread_id":"01900000-0000-7000-8000-000000000001","depth":1,"agent_path":"/root/judge","agent_nickname":"Judge","agent_role":null}}}}}"#;
        let rec: CodexRecord = serde_json::from_str(line).unwrap();
        assert!(matches!(rec, CodexRecord::SessionMeta(_)));
    }

    #[test]
    fn array_tool_output_is_flattened() {
        let line = r#"{"type":"function_call_output","call_id":"c1","output":[{"type":"input_text","text":"a"},{"type":"input_text","text":"b"}]}"#;
        let p: ResponseItemPayload = serde_json::from_str(line).unwrap();
        match p {
            ResponseItemPayload::FunctionCallOutput(o) => assert_eq!(o.output, "a\nb"),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn null_model_context_window_parses() {
        let line = r#"{"type":"token_count","info":{"total_token_usage":{"input_tokens":1,"output_tokens":1,"total_tokens":2},"last_token_usage":{"input_tokens":1,"output_tokens":1,"total_tokens":2},"model_context_window":null},"rate_limits":null}"#;
        let p: EventMsgPayload = serde_json::from_str(line).unwrap();
        match p {
            EventMsgPayload::TokenCount(t) => {
                assert_eq!(t.info.unwrap().model_context_window, None)
            }
            other => panic!("unexpected {other:?}"),
        }
    }
}
