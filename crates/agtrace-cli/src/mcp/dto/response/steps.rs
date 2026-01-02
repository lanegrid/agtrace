use agtrace_sdk::types::AgentSession;
use serde_json::Value;

/// Session steps response (detail_level: steps)
/// Target size: 50-100 KB
/// Returns the full AgentSession but with truncated payloads
pub struct SessionStepsResponse {
    session: AgentSession,
}

impl SessionStepsResponse {
    pub fn from_session(session: AgentSession) -> Self {
        Self { session }
    }

    pub fn into_value(self) -> Result<Value, String> {
        let mut session_value = serde_json::to_value(&self.session)
            .map_err(|e| format!("Serialization error: {}", e))?;

        truncate_session_payloads(&mut session_value);

        if let Some(obj) = session_value.as_object_mut() {
            obj.insert(
                "hint".to_string(),
                Value::String(
                    "Payloads are truncated to 500 chars. Use search_events(pattern) to find specific content, or detail_level='full' for complete data"
                        .to_string(),
                ),
            );
        }

        Ok(session_value)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_reasoning_content() {
        let mut value = serde_json::json!({
            "reasoning": {
                "content": {
                    "text": "a".repeat(1000)
                }
            }
        });

        truncate_reasoning(&mut value, 500);

        let text = value
            .get("reasoning")
            .and_then(|r| r.get("content"))
            .and_then(|c| c.get("text"))
            .and_then(|t| t.as_str())
            .unwrap();

        assert!(text.len() <= 500, "Text should be truncated to 500 chars");
        assert!(text.ends_with("..."), "Truncated text should end with ...");
    }

    #[test]
    fn test_truncate_reasoning_preserves_short_content() {
        let mut value = serde_json::json!({
            "reasoning": {
                "content": {
                    "text": "short text"
                }
            }
        });

        let original = value.clone();
        truncate_reasoning(&mut value, 500);

        assert_eq!(value, original, "Short content should not be modified");
    }

    #[test]
    fn test_truncate_tool_execution_result() {
        let mut value = serde_json::json!({
            "tools": [{
                "result": {
                    "content": {
                        "content": "b".repeat(1000)
                    }
                }
            }]
        });

        truncate_tool_executions(&mut value, 500);

        let content = value
            .get("tools")
            .and_then(|t| t.as_array())
            .and_then(|arr| arr.first())
            .and_then(|tool| tool.get("result"))
            .and_then(|r| r.get("content"))
            .and_then(|c| c.get("content"))
            .and_then(|c| c.as_str())
            .unwrap();

        assert!(
            content.len() <= 500,
            "Content should be truncated to 500 chars"
        );
        assert!(
            content.ends_with("..."),
            "Truncated content should end with ..."
        );
    }

    #[test]
    fn test_truncate_multiple_tool_executions() {
        let mut value = serde_json::json!({
            "tools": [
                {
                    "result": {
                        "content": {
                            "content": "a".repeat(1000)
                        }
                    }
                },
                {
                    "result": {
                        "content": {
                            "content": "b".repeat(1000)
                        }
                    }
                }
            ]
        });

        truncate_tool_executions(&mut value, 500);

        let tools = value.get("tools").and_then(|t| t.as_array()).unwrap();
        assert_eq!(tools.len(), 2);

        for tool in tools {
            let content = tool
                .get("result")
                .and_then(|r| r.get("content"))
                .and_then(|c| c.get("content"))
                .and_then(|c| c.as_str())
                .unwrap();
            assert!(content.len() <= 500);
            assert!(content.ends_with("..."));
        }
    }

    #[test]
    fn test_truncate_session_payloads_integration() {
        let mut value = serde_json::json!({
            "turns": [{
                "steps": [{
                    "reasoning": {
                        "content": {
                            "text": "a".repeat(1000)
                        }
                    },
                    "tools": [{
                        "result": {
                            "content": {
                                "content": "b".repeat(1000)
                            }
                        }
                    }]
                }]
            }]
        });

        truncate_session_payloads(&mut value);

        let reasoning_text = value
            .get("turns")
            .and_then(|t| t.as_array())
            .and_then(|arr| arr.first())
            .and_then(|turn| turn.get("steps"))
            .and_then(|s| s.as_array())
            .and_then(|arr| arr.first())
            .and_then(|step| step.get("reasoning"))
            .and_then(|r| r.get("content"))
            .and_then(|c| c.get("text"))
            .and_then(|t| t.as_str())
            .unwrap();

        let tool_content = value
            .get("turns")
            .and_then(|t| t.as_array())
            .and_then(|arr| arr.first())
            .and_then(|turn| turn.get("steps"))
            .and_then(|s| s.as_array())
            .and_then(|arr| arr.first())
            .and_then(|step| step.get("tools"))
            .and_then(|t| t.as_array())
            .and_then(|arr| arr.first())
            .and_then(|tool| tool.get("result"))
            .and_then(|r| r.get("content"))
            .and_then(|c| c.get("content"))
            .and_then(|c| c.as_str())
            .unwrap();

        assert!(reasoning_text.len() <= 500);
        assert!(reasoning_text.ends_with("..."));
        assert!(tool_content.len() <= 500);
        assert!(tool_content.ends_with("..."));
    }

    #[test]
    fn test_truncate_str_preserves_short_strings() {
        let short = "hello";
        assert_eq!(truncate_str(short, 100), "hello");
    }

    #[test]
    fn test_truncate_str_adds_ellipsis() {
        let long = "a".repeat(100);
        let truncated = truncate_str(&long, 50);

        assert_eq!(truncated.len(), 50);
        assert!(truncated.ends_with("..."));
    }

    #[test]
    fn test_truncate_str_char_boundary() {
        let emoji_str = "😀".repeat(100);
        let truncated = truncate_str(&emoji_str, 50);

        assert_eq!(truncated.chars().count(), 50);
        assert!(truncated.ends_with("..."));
        assert!(truncated.is_char_boundary(truncated.len()));
    }

    #[test]
    fn test_steps_response_includes_hint() {
        use agtrace_sdk::types::SessionStats;
        use chrono::Utc;
        use uuid::Uuid;

        let session = agtrace_sdk::types::AgentSession {
            session_id: Uuid::new_v4(),
            start_time: Utc::now(),
            end_time: Some(Utc::now()),
            turns: vec![],
            stats: SessionStats::default(),
        };

        let response = SessionStepsResponse::from_session(session);
        let value = response.into_value().unwrap();

        let hint = value.get("hint").and_then(|h| h.as_str());
        assert!(hint.is_some(), "Hint should be present");
        assert!(
            hint.unwrap().contains("search_events"),
            "Hint should mention search_events"
        );
    }
}
