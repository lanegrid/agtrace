use crate::model::*;
use crate::utils::{project_hash_from_root, truncate};
use serde_json::Value;

pub fn normalize_codex_stream<I>(
    records: I,
    session_id: &str,
    project_root_override: Option<&str>,
) -> Vec<AgentEventV1>
where
    I: IntoIterator<Item = Value>,
{
    let mut events = Vec::new();
    let mut last_user_event_id: Option<String> = None;
    let mut seq: u64 = 0;

    let mut project_root_str: Option<String> = project_root_override.map(|s| s.to_string());
    let mut project_hash: Option<String> = project_root_str.as_deref().map(project_hash_from_root);

    for rec in records {
        seq += 1;

        let ts = rec
            .get("timestamp")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let payload = rec.get("payload").cloned().unwrap_or(Value::Null);
        let payload_obj = payload.as_object();

        // Extract cwd for project_root
        if project_root_str.is_none() {
            if let Some(cwd) = payload_obj
                .and_then(|m| m.get("cwd"))
                .and_then(|v| v.as_str())
            {
                project_root_str = Some(cwd.to_string());
                project_hash = Some(project_hash_from_root(cwd));
            }
        }

        let project_hash_val = project_hash
            .clone()
            .unwrap_or_else(|| "unknown".to_string());

        let mut ev =
            AgentEventV1::new(Source::Codex, project_hash_val, ts.clone(), EventType::Meta);

        ev.session_id = Some(session_id.to_string());
        ev.project_root = project_root_str.clone();

        let p_type = payload_obj
            .and_then(|m| m.get("type"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let p_role = payload_obj
            .and_then(|m| m.get("role"))
            .and_then(|v| v.as_str());

        let p_name = payload_obj
            .and_then(|m| m.get("name"))
            .and_then(|v| v.as_str());

        let p_status = payload_obj
            .and_then(|m| m.get("status"))
            .and_then(|v| v.as_str());

        if let Some(model) = payload_obj
            .and_then(|m| m.get("model"))
            .and_then(|v| v.as_str())
        {
            ev.model = Some(model.to_string());
        }

        if let Some(info) = payload_obj
            .and_then(|m| m.get("info"))
            .and_then(|v| v.as_object())
        {
            if let Some(last) = info.get("last_token_usage").and_then(|v| v.as_object()) {
                ev.tokens_input = last.get("input_tokens").and_then(|v| v.as_u64());
                ev.tokens_output = last.get("output_tokens").and_then(|v| v.as_u64());
                ev.tokens_total = last.get("total_tokens").and_then(|v| v.as_u64());
                ev.tokens_cached = last.get("cached_input_tokens").and_then(|v| v.as_u64());
                ev.tokens_thinking = last.get("reasoning_output_tokens").and_then(|v| v.as_u64());
            }
        }

        if let Some(call_id) = payload_obj
            .and_then(|m| m.get("call_id"))
            .and_then(|v| v.as_str())
        {
            ev.tool_call_id = Some(call_id.to_string());
        }

        ev.event_id = Some(format!("{}#{}", ev.ts, seq));
        ev.parent_event_id = last_user_event_id.clone();

        if p_type == "message" && p_role.is_some() {
            match p_role.unwrap() {
                "user" => {
                    ev.event_type = EventType::UserMessage;
                    ev.role = Some(Role::User);
                    ev.channel = Some(Channel::Chat);
                    ev.parent_event_id = None;
                    last_user_event_id = ev.event_id.clone();
                    ev.text = extract_codex_message_text(&payload);
                }
                "assistant" => {
                    ev.event_type = EventType::AssistantMessage;
                    ev.role = Some(Role::Assistant);
                    ev.channel = Some(Channel::Chat);
                    ev.text = extract_codex_message_text(&payload);
                }
                _ => {
                    ev.event_type = EventType::SystemMessage;
                    ev.role = Some(Role::System);
                    ev.channel = Some(Channel::System);
                    ev.text = extract_codex_message_text(&payload);
                }
            }
        } else if p_type == "reasoning" {
            ev.event_type = EventType::Reasoning;
            ev.role = Some(Role::Assistant);
            ev.channel = Some(Channel::Chat);
            if let Some(summary) = payload_obj.and_then(|m| m.get("summary")) {
                if let Some(arr) = summary.as_array() {
                    let texts: Vec<String> = arr
                        .iter()
                        .filter_map(|s| {
                            s.get("text")
                                .and_then(|v| v.as_str())
                                .map(|t| t.to_string())
                        })
                        .collect();
                    if !texts.is_empty() {
                        ev.text = Some(texts.join("\n"));
                    }
                }
            }
            if ev.text.is_none() {
                ev.text = payload_obj
                    .and_then(|m| m.get("text"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
            }
        } else if p_type == "agent_reasoning" {
            ev.event_type = EventType::Reasoning;
            ev.role = Some(Role::Assistant);
            ev.channel = Some(Channel::Chat);
            ev.text = payload_obj
                .and_then(|m| m.get("text"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
        } else if p_type == "function_call_output" {
            ev.event_type = EventType::ToolResult;
            ev.role = Some(Role::Tool);
            ev.channel = Some(Channel::Terminal);
            ev.tool_status = Some(
                match payload_obj
                    .and_then(|m| m.get("status"))
                    .and_then(|v| v.as_str())
                {
                    Some("completed") => ToolStatus::Success,
                    Some("failed") | Some("error") => ToolStatus::Error,
                    _ => ToolStatus::Unknown,
                },
            );
            ev.text = payload_obj
                .and_then(|m| m.get("output"))
                .and_then(|v| v.as_str())
                .map(|s| truncate(s, 2000));
        } else if p_name.is_some() {
            ev.event_type = EventType::ToolCall;
            ev.role = Some(Role::Assistant);
            ev.channel = match p_name.unwrap() {
                "shell" => Some(Channel::Terminal),
                "apply_patch" => Some(Channel::Editor),
                _ => Some(Channel::Chat),
            };
            ev.tool_name = Some(p_name.unwrap().to_string());
            ev.text = payload_obj
                .and_then(|m| m.get("arguments"))
                .and_then(|v| v.as_str())
                .map(|s| truncate(s, 2000));
        } else if p_status.is_some() {
            ev.event_type = EventType::ToolResult;
            ev.role = Some(Role::Tool);
            ev.channel = Some(Channel::Terminal);
            ev.tool_status = Some(match p_status.unwrap() {
                "completed" => ToolStatus::Success,
                "failed" | "error" => ToolStatus::Error,
                _ => ToolStatus::Unknown,
            });
            ev.text = payload_obj
                .and_then(|m| m.get("output"))
                .and_then(|v| v.as_str())
                .map(|s| truncate(s, 2000));
        } else {
            ev.event_type = EventType::Meta;
            ev.role = Some(Role::System);
            ev.channel = Some(Channel::System);
            ev.text = payload_obj
                .and_then(|m| m.get("text"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
        }

        ev.raw = rec;
        events.push(ev);
    }

    events
}

fn extract_codex_message_text(payload: &Value) -> Option<String> {
    let obj = payload.as_object()?;
    if let Some(content) = obj.get("content") {
        if let Some(arr) = content.as_array() {
            let mut texts = Vec::new();
            for c in arr {
                if let Some(cobj) = c.as_object() {
                    let c_type = cobj.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    if c_type == "input_text" || c_type == "output_text" {
                        if let Some(t) = cobj.get("text").and_then(|v| v.as_str()) {
                            texts.push(t.to_string());
                        }
                    }
                }
            }
            if !texts.is_empty() {
                return Some(truncate(&texts.join("\n"), 2000));
            }
        }
    }
    obj.get("text")
        .and_then(|v| v.as_str())
        .map(|s| truncate(s, 2000))
}
