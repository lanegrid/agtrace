//! Typed Codex rollout records (Codex ≥ 0.153, paginated rollout, multi_agent v2).
//!
//! Decoding is two-stage: every line is first read as an [`Envelope`] whose payload stays
//! raw ([`RawValue`]), then only the payloads the decoder needs are deserialized into the
//! structs below. Large payloads that are never used (`compacted.replacement_history`,
//! `world_state`, fork prefixes) are therefore never materialized.
//!
//! Every field that is not needed to identify a record is optional so that schema drift
//! degrades to missing data instead of a failed line.

use serde::Deserialize;
use serde_json::Value;
use serde_json::value::RawValue;
use std::borrow::Cow;
use std::collections::BTreeMap;

/// `{type, ordinal, timestamp, payload}` of one rollout line.
#[derive(Debug, Deserialize)]
pub(crate) struct Envelope<'a> {
    #[serde(rename = "type", borrow, default)]
    pub kind: Option<Cow<'a, str>>,
    #[serde(default)]
    pub ordinal: Option<u64>,
    #[serde(borrow, default)]
    pub timestamp: Option<Cow<'a, str>>,
    #[serde(borrow, default)]
    pub payload: Option<&'a RawValue>,
}

/// Only the `type` tag of a payload / item.
#[derive(Debug, Deserialize)]
pub(crate) struct TypeTag<'a> {
    #[serde(rename = "type", borrow, default)]
    pub kind: Option<Cow<'a, str>>,
}

// ---------------------------------------------------------------------------
// session_meta / turn_context
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub(crate) struct SessionMeta {
    #[serde(default)]
    pub subagent_history_start_ordinal: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TurnContext {
    #[serde(default)]
    pub turn_id: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
}

// ---------------------------------------------------------------------------
// token usage
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
pub(crate) struct Usage {
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
}

/// Top-level `token_usage_record`: one per model response.
#[derive(Debug, Deserialize)]
pub(crate) struct TokenUsageRecord {
    #[serde(default)]
    pub response_id: Option<String>,
    #[serde(default)]
    pub usage: Option<Usage>,
}

/// `event_msg.token_count` — only the context window is used.
#[derive(Debug, Deserialize)]
pub(crate) struct TokenCount {
    #[serde(default)]
    pub info: Option<TokenCountInfo>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TokenCountInfo {
    #[serde(default)]
    pub model_context_window: Option<u64>,
}

// ---------------------------------------------------------------------------
// event_msg
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub(crate) struct TaskStarted {
    #[serde(default)]
    pub turn_id: Option<String>,
    #[serde(default)]
    pub model_context_window: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TaskComplete {
    #[serde(default)]
    pub turn_id: Option<String>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub last_agent_message: Option<String>,
    #[serde(default)]
    pub error: Option<TaskError>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TaskError {
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TurnAborted {
    #[serde(default)]
    pub turn_id: Option<String>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ThreadSettingsApplied {
    #[serde(default)]
    pub thread_settings: Option<ThreadSettings>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ThreadSettings {
    #[serde(default)]
    pub model: Option<String>,
}

/// `event_msg.item_completed` with the item kept raw (dispatched on `item.type`).
#[derive(Debug, Deserialize)]
pub(crate) struct ItemCompleted {
    #[serde(default)]
    pub turn_id: Option<String>,
    pub item: Box<RawValue>,
}

// ---------------------------------------------------------------------------
// item_completed items
// ---------------------------------------------------------------------------

/// `{secs, nanos}` duration of executed items.
#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub(crate) struct ItemDuration {
    #[serde(default)]
    pub secs: u64,
    #[serde(default)]
    pub nanos: u64,
}

impl ItemDuration {
    pub fn as_millis(&self) -> u64 {
        self.secs * 1000 + self.nanos / 1_000_000
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct CommandExecutionItem {
    /// `["/bin/zsh", "-lc", "<cmd>"]` (or a plain string).
    #[serde(default)]
    pub command: Option<Value>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub exit_code: Option<i32>,
    #[serde(default)]
    pub aggregated_output: Option<String>,
    #[serde(default)]
    pub stdout: Option<String>,
    #[serde(default)]
    pub duration: Option<ItemDuration>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct FileChangeItem {
    #[serde(default)]
    pub changes: BTreeMap<String, FileChangeEntry>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub stdout: Option<String>,
    #[serde(default)]
    pub stderr: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct FileChangeEntry {
    /// add | update | delete
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub unified_diff: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ImageViewItem {
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ExtensionItem {
    /// `web.search`, `clock.sleep`, ...
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub action: Option<Value>,
    #[serde(rename = "durationMs", default)]
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct McpToolCallItem {
    #[serde(default)]
    pub server: Option<String>,
    #[serde(default)]
    pub tool: Option<String>,
    #[serde(default)]
    pub arguments: Option<Value>,
    #[serde(default)]
    pub result: Option<McpResult>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub duration: Option<ItemDuration>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct McpResult {
    #[serde(default)]
    pub content: Vec<ContentItem>,
    #[serde(rename = "isError", default)]
    pub is_error: Option<bool>,
}

/// Lifecycle of a child agent, written in the parent's rollout.
#[derive(Debug, Deserialize)]
pub(crate) struct SubAgentActivityItem {
    /// spawn / interact / interrupt call_id, or `subagent-completed-<uuid>`.
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub agent_path: Option<String>,
    #[serde(default)]
    pub agent_thread_id: Option<String>,
    /// started | interacted | interrupted | completed
    #[serde(default)]
    pub kind: Option<String>,
}

// ---------------------------------------------------------------------------
// response_item
// ---------------------------------------------------------------------------

/// Generic `{type, text}` content item (input_text, output_text, summary_text, text, ...).
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ContentItem {
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub encrypted_content: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct Passthrough {
    #[serde(default)]
    pub content_item_kinds: Option<Vec<String>>,
    #[serde(default)]
    pub turn_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MessageItem {
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub content: Vec<ContentItem>,
    #[serde(default)]
    pub phase: Option<String>,
    #[serde(default)]
    pub internal_chat_message_metadata_passthrough: Option<Passthrough>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ReasoningItem {
    #[serde(default)]
    pub summary: Vec<ContentItem>,
    #[serde(default)]
    pub content: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct FunctionCallItem {
    pub name: String,
    #[serde(default)]
    pub namespace: Option<String>,
    #[serde(default)]
    pub arguments: Option<String>,
    pub call_id: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ToolOutputItem {
    pub call_id: String,
    /// String or content-item array (flattened).
    #[serde(default, deserialize_with = "crate::lenient::deserialize_flat_output")]
    pub output: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CustomToolCallItem {
    pub name: String,
    pub call_id: String,
    #[serde(default)]
    pub input: String,
    #[serde(default)]
    pub internal_chat_message_metadata_passthrough: Option<Passthrough>,
}

/// Inter-agent message received by this thread.
#[derive(Debug, Deserialize)]
pub(crate) struct AgentMessageItem {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub recipient: Option<String>,
    #[serde(default)]
    pub content: Vec<ContentItem>,
}

// ---------------------------------------------------------------------------
// other top-level records
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub(crate) struct InterAgentCommunicationMetadata {
    #[serde(default)]
    pub trigger_turn: Option<bool>,
}

/// `compacted` — `replacement_history` / `guardian_history` are deliberately not modeled.
#[derive(Debug, Deserialize)]
pub(crate) struct Compacted {
    #[serde(default)]
    pub window_number: Option<u32>,
    #[serde(default)]
    pub latest_token_usage_record: Option<TokenUsageRecord>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_keeps_payload_raw() {
        let line = r#"{"timestamp":"2026-09-20T10:00:00.000Z","type":"compacted","ordinal":7,"payload":{"window_number":2,"replacement_history":[{"huge":true}],"latest_token_usage_record":{"response_id":"resp_synthetic_1","usage":{"input_tokens":1000}}}}"#;
        let env: Envelope = serde_json::from_str(line).unwrap();
        assert_eq!(env.kind.as_deref(), Some("compacted"));
        assert_eq!(env.ordinal, Some(7));
        let c: Compacted = serde_json::from_str(env.payload.unwrap().get()).unwrap();
        assert_eq!(c.window_number, Some(2));
        assert_eq!(
            c.latest_token_usage_record
                .and_then(|r| r.usage)
                .map(|u| u.input_tokens),
            Some(1000)
        );
    }

    #[test]
    fn escaped_type_tag_is_supported() {
        let env: Envelope = serde_json::from_str(r#"{"type":"event_msg"}"#).unwrap();
        assert_eq!(env.kind.as_deref(), Some("event_msg"));
    }
}
