use crate::model::*;
use crate::utils::truncate;
use serde_json::Value;

use super::schema::{GeminiMessage, GeminiSession};

pub fn normalize_gemini_session(session: &GeminiSession) -> Vec<AgentEventV1> {
    let mut events = Vec::new();
    let mut last_user_event_id: Option<String> = None;

    let project_hash = &session.project_hash;
    let session_id = Some(&session.session_id);

    for msg in &session.messages {
        match msg {
            GeminiMessage::User(user_msg) => {
                let ts = &user_msg.timestamp;
                let msg_id = Some(&user_msg.id);
                let mut ev = AgentEventV1::new(
                    Source::Gemini,
                    project_hash.clone(),
                    ts.clone(),
                    EventType::UserMessage,
                );
                ev.session_id = session_id.cloned();
                ev.event_id = msg_id.cloned();
                ev.parent_event_id = None;
                ev.role = Some(Role::User);
                ev.channel = Some(Channel::Chat);
                ev.model = None;
                ev.text = Some(user_msg.content.clone());
                last_user_event_id = ev.event_id.clone();
                ev.raw = serde_json::to_value(user_msg).unwrap_or(Value::Null);
                events.push(ev);
            }
            GeminiMessage::Gemini(gemini_msg) => {
                let ts = &gemini_msg.timestamp;
                let msg_id = Some(&gemini_msg.id);
                let model = Some(&gemini_msg.model);

                let mut ev = AgentEventV1::new(
                    Source::Gemini,
                    project_hash.clone(),
                    ts.clone(),
                    EventType::AssistantMessage,
                );
                ev.session_id = session_id.cloned();
                ev.event_id = msg_id.cloned();
                ev.parent_event_id = last_user_event_id.clone();
                ev.role = Some(Role::Assistant);
                ev.channel = Some(Channel::Chat);
                ev.model = model.cloned();
                ev.text = Some(gemini_msg.content.clone());
                ev.raw = serde_json::to_value(gemini_msg).unwrap_or(Value::Null);
                events.push(ev);

                for (idx, thought) in gemini_msg.thoughts.iter().enumerate() {
                    let mut rev = AgentEventV1::new(
                        Source::Gemini,
                        project_hash.clone(),
                        ts.clone(),
                        EventType::Reasoning,
                    );
                    rev.session_id = session_id.cloned();
                    rev.event_id = Some(format!(
                        "{}#thought{}",
                        msg_id.unwrap_or(&"".to_string()),
                        idx
                    ));
                    rev.parent_event_id = last_user_event_id.clone();
                    rev.role = Some(Role::Assistant);
                    rev.channel = Some(Channel::Chat);
                    rev.model = model.cloned();
                    rev.text = Some(format!("{}: {}", thought.subject, thought.description));
                    rev.raw = serde_json::to_value(thought).unwrap_or(Value::Null);
                    events.push(rev);
                }

                for tool_call in &gemini_msg.tool_calls {
                    let mut tev = AgentEventV1::new(
                        Source::Gemini,
                        project_hash.clone(),
                        ts.clone(),
                        EventType::ToolCall,
                    );
                    tev.session_id = session_id.cloned();
                    tev.event_id = Some(tool_call.id.clone());
                    tev.tool_call_id = tev.event_id.clone();
                    tev.parent_event_id = last_user_event_id.clone();
                    tev.role = Some(Role::Assistant);
                    tev.channel = Some(Channel::Terminal);
                    tev.model = model.cloned();
                    tev.tool_name = Some(tool_call.name.clone());
                    if let Some(status) = &tool_call.status {
                        tev.tool_status = Some(match status.as_str() {
                            "success" => ToolStatus::Success,
                            "error" => ToolStatus::Error,
                            _ => ToolStatus::Unknown,
                        });
                    }
                    let args = serde_json::to_string(&tool_call.args).unwrap_or_default();
                    let result_display = tool_call.result_display.as_deref().unwrap_or("");
                    if !result_display.is_empty() {
                        tev.text = Some(truncate(result_display, 1000));
                    } else if !args.is_empty() {
                        tev.text = Some(truncate(&args, 500));
                    }
                    tev.raw = serde_json::to_value(tool_call).unwrap_or(Value::Null);
                    events.push(tev);
                }
            }
            GeminiMessage::Info(info_msg) => {
                let ts = &info_msg.timestamp;
                let msg_id = Some(&info_msg.id);

                let mut ev = AgentEventV1::new(
                    Source::Gemini,
                    project_hash.clone(),
                    ts.clone(),
                    EventType::SystemMessage,
                );
                ev.session_id = session_id.cloned();
                ev.event_id = msg_id.cloned();
                ev.parent_event_id = last_user_event_id.clone();
                ev.role = Some(Role::System);
                ev.channel = Some(Channel::System);
                ev.text = Some(info_msg.content.clone());
                ev.raw = serde_json::to_value(info_msg).unwrap_or(Value::Null);
                events.push(ev);
            }
        }
    }
    events
}
