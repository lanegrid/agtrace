use serde::{Deserialize, Serialize};

/// Detail level for session responses
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DetailLevel {
    /// Only session stats and turn metadata (5-10 KB)
    Summary,
    /// Turn-level summaries with tool outcomes (15-30 KB)
    Turns,
    /// Step-level details with truncated payloads (50-100 KB)
    Steps,
    /// Complete session with all payloads (unbounded, use with caution)
    Full,
}

impl Default for DetailLevel {
    fn default() -> Self {
        Self::Summary
    }
}

/// Truncate a string to a maximum length, adding ellipsis if truncated
pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s.chars().take(max_len - 3).collect::<String>())
    } else {
        s.to_string()
    }
}

/// Truncate a JSON value recursively
pub fn truncate_json_value(value: &serde_json::Value, max_string_len: usize) -> serde_json::Value {
    match value {
        serde_json::Value::String(s) => {
            serde_json::Value::String(truncate_string(s, max_string_len))
        }
        serde_json::Value::Array(arr) => serde_json::Value::Array(
            arr.iter()
                .take(3)
                .map(|v| truncate_json_value(v, max_string_len))
                .collect(),
        ),
        serde_json::Value::Object(obj) => serde_json::Value::Object(
            obj.iter()
                .take(5)
                .map(|(k, v)| (k.clone(), truncate_json_value(v, max_string_len)))
                .collect(),
        ),
        _ => value.clone(),
    }
}
