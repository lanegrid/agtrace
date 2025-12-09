use crate::model::*;
use crate::utils::*;
use anyhow::{Context, Result};
use serde_json::Value;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Parse Codex JSONL file and normalize to AgentEventV1
pub fn normalize_codex_file(
    path: &Path,
    fallback_session_id: &str,
    project_root_override: Option<&str>,
) -> Result<Vec<AgentEventV1>> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read Codex file: {}", path.display()))?;

    let mut records: Vec<Value> = Vec::new();
    let mut session_id_from_meta: Option<String> = None;

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let v: Value = serde_json::from_str(line)
            .with_context(|| format!("Failed to parse JSON line: {}", line))?;

        // Extract session_id from session_meta record (Spec 2.5.5)
        if v.get("type").and_then(|t| t.as_str()) == Some("session_meta") {
            if let Some(id) = v
                .get("payload")
                .and_then(|p| p.get("id"))
                .and_then(|id| id.as_str())
            {
                session_id_from_meta = Some(id.to_string());
            }
        }

        records.push(v);
    }

    // Use session_meta.payload.id if available, otherwise fallback to filename-based ID
    let session_id = session_id_from_meta
        .as_deref()
        .unwrap_or(fallback_session_id);

    Ok(normalize_codex_stream(
        records.into_iter(),
        session_id,
        project_root_override,
    ))
}

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

        // Extract payload fields
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

        let p_model = payload_obj
            .and_then(|m| m.get("model"))
            .and_then(|v| v.as_str());

        if let Some(model) = p_model {
            ev.model = Some(model.to_string());
        }

        // Extract token information
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

        // Tool call ID (for linking call and result)
        if let Some(call_id) = payload_obj
            .and_then(|m| m.get("call_id"))
            .and_then(|v| v.as_str())
        {
            ev.tool_call_id = Some(call_id.to_string());
        }

        // Event ID - always unique
        ev.event_id = Some(format!("{}#{}", ev.ts, seq));

        ev.parent_event_id = last_user_event_id.clone();

        // Determine event type and properties
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
            // Extract text from summary if available
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
            // Fallback to text field
            if ev.text.is_none() {
                ev.text = payload_obj
                    .and_then(|m| m.get("text"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
            }
        } else if p_type == "agent_reasoning" {
            // agent_reasoning should also be mapped to Reasoning
            ev.event_type = EventType::Reasoning;
            ev.role = Some(Role::Assistant);
            ev.channel = Some(Channel::Chat);
            ev.text = payload_obj
                .and_then(|m| m.get("text"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
        } else if p_type == "function_call_output" {
            // function_call_output should be mapped to ToolResult
            ev.event_type = EventType::ToolResult;
            ev.role = Some(Role::Tool); // v1.5: tool_result role is Tool
            ev.channel = Some(Channel::Terminal); // Assume terminal for shell commands

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
            // Tool call
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
            // Tool result
            ev.event_type = EventType::ToolResult;
            ev.role = Some(Role::Tool); // v1.5: tool_result role is Tool
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
            // Meta event
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

/// Extract cwd from a Codex session file by reading the first few records
/// Returns None if cwd cannot be determined
pub fn extract_cwd_from_codex_file(path: &Path) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    let reader = BufReader::new(file);

    // Read first 10 lines to find cwd in payload
    for line in reader.lines().take(10) {
        if let Ok(line) = line {
            if let Ok(json) = serde_json::from_str::<Value>(&line) {
                if let Some(payload) = json.get("payload") {
                    if let Some(cwd) = payload.get("cwd").and_then(|v| v.as_str()) {
                        return Some(cwd.to_string());
                    }
                }
            }
        }
    }

    None
}

/// Check if a Codex session file is empty or incomplete (has only metadata, no actual events)
pub fn is_empty_codex_session(path: &Path) -> bool {
    let Ok(file) = std::fs::File::open(path) else {
        return true;
    };
    let reader = BufReader::new(file);

    let mut line_count = 0;
    let mut has_event = false;

    // Check first 20 lines
    for line in reader.lines().take(20) {
        if let Ok(line) = line {
            line_count += 1;
            if let Ok(json) = serde_json::from_str::<Value>(&line) {
                // If there's a payload with actual event data (not just session metadata)
                if let Some(payload) = json.get("payload") {
                    // Check if this looks like an actual event (has cwd, or text, or other event fields)
                    if payload.get("cwd").is_some()
                        || payload.get("text").is_some()
                        || payload.get("content").is_some()
                    {
                        has_event = true;
                        break;
                    }
                }
            }
        }
    }

    // If we only have 1-2 lines and no events, it's likely an empty session
    line_count <= 2 && !has_event
}

pub struct CodexProvider;

impl CodexProvider {
    pub fn new() -> Self {
        Self
    }
}

impl super::LogProvider for CodexProvider {
    fn name(&self) -> &str {
        "codex"
    }

    fn can_handle(&self, path: &Path) -> bool {
        if !path.is_file() {
            return false;
        }

        if !path.extension().map_or(false, |e| e == "jsonl") {
            return false;
        }

        let filename = path.file_name().and_then(|f| f.to_str()).unwrap_or("");
        if !filename.starts_with("rollout-") {
            return false;
        }

        if is_empty_codex_session(path) {
            return false;
        }

        true
    }

    fn normalize_file(&self, path: &Path, context: &super::ImportContext) -> Result<Vec<AgentEventV1>> {
        let filename = path
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("Invalid file path"))?
            .to_string_lossy();

        let session_id_base = if filename.ends_with(".jsonl") {
            &filename[..filename.len() - 6]
        } else {
            filename.as_ref()
        };

        let session_id = context
            .session_id_prefix
            .as_ref()
            .map(|p| format!("{}{}", p, session_id_base))
            .unwrap_or_else(|| session_id_base.to_string());

        normalize_codex_file(path, &session_id, context.project_root_override.as_deref())
    }

    fn belongs_to_project(&self, path: &Path, target_project_root: &Path) -> bool {
        use crate::utils::paths_equal;

        if let Some(session_cwd) = extract_cwd_from_codex_file(path) {
            let session_cwd_path = Path::new(&session_cwd);
            paths_equal(target_project_root, session_cwd_path)
        } else {
            false
        }
    }
}
