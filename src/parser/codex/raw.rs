use serde::Deserialize;
use serde_json::Value;

/// Codex event structure (JSONL format)
#[cfg_attr(test, derive(serde::Serialize))]
#[cfg_attr(test, serde(deny_unknown_fields))]
#[derive(Debug, Deserialize)]
pub struct CodexEvent {
    pub(crate) timestamp: String,
    #[serde(rename = "type")]
    pub(crate) event_type: String,
    pub(crate) payload: Value,
}
