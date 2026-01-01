use agtrace_sdk::types::AgentSession;
use serde_json::Value;

/// Session steps response (detail_level: steps)
/// Target size: 50-100 KB
/// Returns the full AgentSession but with truncated payloads
pub struct SessionStepsResponse {
    session_value: Value,
}

impl SessionStepsResponse {
    pub fn from_session(session: AgentSession) -> Result<Self, String> {
        let mut session_value =
            serde_json::to_value(&session).map_err(|e| format!("Serialization error: {}", e))?;

        truncate_session_payloads(&mut session_value);

        Ok(Self { session_value })
    }

    pub fn into_value(self) -> Value {
        self.session_value
    }
}

/// Truncate large payloads in the session JSON
fn truncate_session_payloads(value: &mut Value) {
    const MAX_PAYLOAD_LEN: usize = 500;

    if let Some(turns) = value.get_mut("turns").and_then(|v| v.as_array_mut()) {
        for turn in turns {
            if let Some(steps) = turn.get_mut("steps").and_then(|v| v.as_array_mut()) {
                for step in steps {
                    truncate_reasoning(step, MAX_PAYLOAD_LEN);
                    truncate_tool_executions(step, MAX_PAYLOAD_LEN);
                }
            }
        }
    }
}

fn truncate_reasoning(step: &mut Value, max_len: usize) {
    if let Some(reasoning) = step.get_mut("reasoning")
        && let Some(content) = reasoning.get_mut("content")
        && let Some(text) = content.get_mut("text").and_then(|v| v.as_str())
        && text.len() > max_len
    {
        *content.get_mut("text").unwrap() = Value::String(truncate_str(text, max_len));
    }
}

fn truncate_tool_executions(step: &mut Value, max_len: usize) {
    if let Some(tools) = step.get_mut("tools").and_then(|v| v.as_array_mut()) {
        for tool_exec in tools {
            if let Some(result) = tool_exec.get_mut("result")
                && let Some(content) = result.get_mut("content")
                && let Some(text) = content.get_mut("content").and_then(|v| v.as_str())
                && text.len() > max_len
            {
                *content.get_mut("content").unwrap() = Value::String(truncate_str(text, max_len));
            }
        }
    }
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s.chars().take(max_len - 3).collect::<String>())
    } else {
        s.to_string()
    }
}
