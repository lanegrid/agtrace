use crate::model::*;
use crate::utils::*;
use anyhow::{Context, Result};
use serde_json::Value;
use std::path::Path;

/// Parse Gemini CLI JSON file and normalize to AgentEventV1
pub fn normalize_gemini_file(path: &Path) -> Result<Vec<AgentEventV1>> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read Gemini file: {}", path.display()))?;

    let json: Value = serde_json::from_str(&text)
        .with_context(|| format!("Failed to parse Gemini JSON: {}", path.display()))?;

    Ok(normalize_gemini_session(&json))
}

pub fn normalize_gemini_session(session: &Value) -> Vec<AgentEventV1> {
    let mut events = Vec::new();
    let mut last_user_event_id: Option<String> = None;

    let project_hash = session
        .get("projectHash")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let session_id = session
        .get("sessionId")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let messages = session.get("messages").and_then(|v| v.as_array());

    if let Some(msgs) = messages {
        for msg in msgs {
            let msg_id = msg
                .get("id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let msg_type = msg.get("type").and_then(|v| v.as_str()).unwrap_or("");
            let ts = msg
                .get("timestamp")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let model = msg
                .get("model")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            match msg_type {
                "user" => {
                    // User message
                    let mut ev = AgentEventV1::new(
                        Source::Gemini,
                        project_hash.clone(),
                        ts.clone(),
                        EventType::UserMessage,
                    );

                    ev.session_id = session_id.clone();
                    ev.event_id = msg_id.clone();
                    ev.parent_event_id = None;
                    ev.role = Some(Role::User);
                    ev.channel = Some(Channel::Chat);
                    ev.model = model.clone();

                    ev.text = msg
                        .get("content")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    last_user_event_id = ev.event_id.clone();
                    ev.raw = msg.clone();
                    events.push(ev);
                }
                "gemini" => {
                    // Assistant message
                    let mut ev = AgentEventV1::new(
                        Source::Gemini,
                        project_hash.clone(),
                        ts.clone(),
                        EventType::AssistantMessage,
                    );

                    ev.session_id = session_id.clone();
                    ev.event_id = msg_id.clone();
                    ev.parent_event_id = last_user_event_id.clone();
                    ev.role = Some(Role::Assistant);
                    ev.channel = Some(Channel::Chat);
                    ev.model = model.clone();

                    ev.text = msg
                        .get("content")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    ev.raw = msg.clone();
                    events.push(ev);

                    // Extract thoughts as reasoning events
                    if let Some(thoughts) = msg.get("thoughts").and_then(|v| v.as_array()) {
                        for (idx, thought) in thoughts.iter().enumerate() {
                            let mut rev = AgentEventV1::new(
                                Source::Gemini,
                                project_hash.clone(),
                                ts.clone(),
                                EventType::Reasoning,
                            );

                            rev.session_id = session_id.clone();
                            rev.event_id = Some(format!(
                                "{}#thought{}",
                                msg_id.as_ref().unwrap_or(&"".to_string()),
                                idx
                            ));
                            rev.parent_event_id = last_user_event_id.clone();
                            rev.role = Some(Role::Assistant);
                            rev.channel = Some(Channel::Chat);
                            rev.model = model.clone();

                            let subject = thought
                                .get("subject")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            let description = thought
                                .get("description")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            rev.text = Some(format!("{}: {}", subject, description));

                            rev.raw = thought.clone();
                            events.push(rev);
                        }
                    }

                    // Extract tool calls
                    if let Some(tool_calls) = msg.get("toolCalls").and_then(|v| v.as_array()) {
                        for tool_call in tool_calls {
                            let mut tev = AgentEventV1::new(
                                Source::Gemini,
                                project_hash.clone(),
                                ts.clone(),
                                EventType::ToolCall,
                            );

                            tev.session_id = session_id.clone();
                            tev.event_id = tool_call
                                .get("id")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());
                            tev.tool_call_id = tev.event_id.clone();
                            tev.parent_event_id = last_user_event_id.clone();
                            tev.role = Some(Role::Assistant);
                            tev.channel = Some(Channel::Terminal);
                            tev.model = model.clone();

                            tev.tool_name = tool_call
                                .get("name")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());

                            // Status
                            if let Some(status) = tool_call.get("status").and_then(|v| v.as_str()) {
                                tev.tool_status = Some(match status {
                                    "success" => ToolStatus::Success,
                                    "error" => ToolStatus::Error,
                                    _ => ToolStatus::Unknown,
                                });
                            }

                            // Args and result
                            let args = tool_call.get("args").and_then(|v| v.as_str()).unwrap_or("");
                            let result_display = tool_call
                                .get("resultDisplay")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");

                            if !result_display.is_empty() {
                                tev.text = Some(truncate(result_display, 1000));
                            } else if !args.is_empty() {
                                tev.text = Some(truncate(args, 500));
                            }

                            tev.raw = tool_call.clone();
                            events.push(tev);
                        }
                    }
                }
                "info" => {
                    // Info/system message
                    let mut ev = AgentEventV1::new(
                        Source::Gemini,
                        project_hash.clone(),
                        ts.clone(),
                        EventType::SystemMessage,
                    );

                    ev.session_id = session_id.clone();
                    ev.event_id = msg_id.clone();
                    ev.parent_event_id = last_user_event_id.clone();
                    ev.role = Some(Role::System);
                    ev.channel = Some(Channel::System);

                    ev.text = msg
                        .get("content")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    ev.raw = msg.clone();
                    events.push(ev);
                }
                _ => {}
            }
        }
    }

    events
}

/// Extract projectHash from a Gemini logs.json file
/// Returns None if projectHash cannot be determined
pub fn extract_project_hash_from_gemini_file(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let json: Value = serde_json::from_str(&text).ok()?;

    json.get("projectHash")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}
