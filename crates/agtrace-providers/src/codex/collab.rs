//! Codex multi_agent v2 collaboration tools and inter-agent messages.
//!
//! Agents are addressed by `agent_path` (`/root`, `/root/judge`). Message bodies sent via
//! `spawn_agent` / `send_message` / `followup_task` are encrypted (`gAAAA…` tokens);
//! only `FINAL_ANSWER` messages are plaintext.

use agtrace_types::{AgentMessageKind, AgentOp, AgentToolArgs};
use serde_json::Value;

use super::records::ContentItem;

/// Path of a root thread.
pub(crate) const ROOT_PATH: &str = "/root";

/// Maximum size of a plaintext message body kept on events.
pub(crate) const MAX_BODY_BYTES: usize = 16 * 1024;
/// Maximum size of a message preview kept on tool-call arguments.
const MAX_PREVIEW_BYTES: usize = 16 * 1024;

/// Agent operation of a collaboration tool (`namespace: "collaboration"`).
pub(crate) fn agent_op(name: &str) -> Option<AgentOp> {
    Some(match name {
        "spawn_agent" => AgentOp::Spawn,
        "send_message" => AgentOp::Send,
        "followup_task" => AgentOp::Followup,
        "interrupt_agent" => AgentOp::Interrupt,
        "wait_agent" => AgentOp::Wait,
        "list_agents" => AgentOp::List,
        _ => return None,
    })
}

/// Kind of the outgoing message a collaboration call sends, if any.
pub(crate) fn outgoing_message_kind(name: &str) -> Option<AgentMessageKind> {
    Some(match name {
        "spawn_agent" | "followup_task" => AgentMessageKind::NewTask,
        "send_message" => AgentMessageKind::Message,
        "interrupt_agent" => AgentMessageKind::Interrupt,
        _ => return None,
    })
}

/// Encrypted message tokens (Fernet-like).
pub(crate) fn is_encrypted_token(s: &str) -> bool {
    s.starts_with("gAAAA")
}

/// Truncate to at most `max` bytes on a char boundary.
pub(crate) fn truncate_bytes(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut end = max;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    s[..end].to_string()
}

/// Plaintext message body: `(body, encrypted)`.
pub(crate) fn message_body(message: Option<&str>) -> (Option<String>, bool) {
    match message {
        Some(m) if is_encrypted_token(m) => (None, true),
        Some(m) if !m.is_empty() => (Some(truncate_bytes(m, MAX_BODY_BYTES)), false),
        _ => (None, false),
    }
}

fn str_arg(args: &Value, key: &str) -> Option<String> {
    args.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// Normalized arguments of a collaboration tool call.
pub(crate) fn agent_tool_args(name: &str, args: &Value) -> Option<AgentToolArgs> {
    let op = agent_op(name)?;
    let mut out = AgentToolArgs::new(op);
    let message = args.get("message").and_then(Value::as_str);
    let (body, encrypted) = message_body(message);
    out.encrypted = encrypted;
    out.message_preview = body.map(|b| truncate_bytes(&b, MAX_PREVIEW_BYTES));
    match op {
        AgentOp::Spawn => {
            out.name = str_arg(args, "task_name");
            out.model = str_arg(args, "model");
            out.fork = args
                .get("fork_turns")
                .and_then(Value::as_str)
                .map(|f| f == "all");
        }
        _ => out.target = str_arg(args, "target"),
    }
    Some(out)
}

/// Resolve an agent target (`judge`, `/root/judge`) relative to the sender's path.
///
/// Bare names address children of the sender (`/root` + `judge` → `/root/judge`).
pub(crate) fn resolve_target(self_path: &str, target: &str) -> String {
    let target = target.trim();
    if target.starts_with('/') {
        return target.to_string();
    }
    if target == "root" {
        return ROOT_PATH.to_string();
    }
    format!("{}/{}", self_path.trim_end_matches('/'), target)
}

/// Parent path of an agent path (`/root/a/b` → `/root/a`); None for `/root`.
pub(crate) fn parent_path(path: &str) -> Option<String> {
    let trimmed = path.trim_end_matches('/');
    let (parent, _) = trimmed.rsplit_once('/')?;
    (!parent.is_empty()).then(|| parent.to_string())
}

/// Last segment of an agent path (`/root/judge` → `judge`).
pub(crate) fn path_leaf(path: &str) -> Option<String> {
    path.rsplit('/')
        .next()
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// Parsed `response_item.agent_message` content.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ReceivedMessage {
    pub kind: AgentMessageKind,
    pub body: Option<String>,
    pub encrypted: bool,
}

/// Parse the `Message Type: X … Payload:\n<body>` header and the encrypted part.
pub(crate) fn parse_received_message(content: &[ContentItem]) -> ReceivedMessage {
    let encrypted = content
        .iter()
        .any(|c| c.kind.as_deref() == Some("encrypted_content") || c.encrypted_content.is_some());
    let text = content
        .iter()
        .filter_map(|c| c.text.as_deref())
        .collect::<Vec<_>>()
        .join("");

    let kind = text
        .lines()
        .find_map(|l| l.strip_prefix("Message Type:"))
        .map(str::trim)
        .map(|t| match t {
            "MESSAGE" => AgentMessageKind::Message,
            "NEW_TASK" => AgentMessageKind::NewTask,
            "FINAL_ANSWER" => AgentMessageKind::FinalAnswer,
            other => AgentMessageKind::Other(other.to_string()),
        })
        .unwrap_or(AgentMessageKind::Message);

    let body = if encrypted {
        None
    } else {
        let payload = match text.find("Payload:") {
            Some(idx) => text[idx + "Payload:".len()..].trim_start_matches(['\r', '\n']),
            None => text.as_str(),
        };
        let payload = payload.trim_end();
        (!payload.is_empty()).then(|| truncate_bytes(payload, MAX_BODY_BYTES))
    };

    ReceivedMessage {
        kind,
        body,
        encrypted,
    }
}

/// True when a `spawn_agent` output reports a failure instead of `{"task_name": …}`.
pub(crate) fn is_failed_spawn_output(output: &str) -> bool {
    serde_json::from_str::<Value>(output)
        .ok()
        .and_then(|v| v.get("task_name").and_then(Value::as_str).map(|_| ()))
        .is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(kind: &str, text: Option<&str>, enc: Option<&str>) -> ContentItem {
        ContentItem {
            kind: Some(kind.into()),
            text: text.map(Into::into),
            encrypted_content: enc.map(Into::into),
        }
    }

    #[test]
    fn spawn_args() {
        let args = serde_json::json!({"task_name":"judge","message":"gAAAA_SYNTHETIC_1","fork_turns":"all","model":"gpt-5.6-sol"});
        let a = agent_tool_args("spawn_agent", &args).unwrap();
        assert_eq!(a.op, AgentOp::Spawn);
        assert_eq!(a.name.as_deref(), Some("judge"));
        assert_eq!(a.model.as_deref(), Some("gpt-5.6-sol"));
        assert_eq!(a.fork, Some(true));
        assert!(a.encrypted);
        assert_eq!(a.message_preview, None);
    }

    #[test]
    fn send_args_keep_target() {
        let a = agent_tool_args(
            "send_message",
            &serde_json::json!({"target":"judge","message":"gAAAA_x"}),
        )
        .unwrap();
        assert_eq!(a.op, AgentOp::Send);
        assert_eq!(a.target.as_deref(), Some("judge"));
        assert!(agent_tool_args("wait", &serde_json::json!({})).is_none());
    }

    #[test]
    fn target_resolution() {
        assert_eq!(resolve_target("/root", "judge"), "/root/judge");
        assert_eq!(resolve_target("/root", "/root/judge"), "/root/judge");
        assert_eq!(resolve_target("/root/a", "b"), "/root/a/b");
        assert_eq!(resolve_target("/root/a", "root"), "/root");
        assert_eq!(parent_path("/root/a/b").as_deref(), Some("/root/a"));
        assert_eq!(parent_path("/root/a").as_deref(), Some("/root"));
        assert_eq!(parent_path("/root"), None);
        assert_eq!(path_leaf("/root/judge").as_deref(), Some("judge"));
    }

    #[test]
    fn encrypted_new_task() {
        let m = parse_received_message(&[
            item(
                "input_text",
                Some("Message Type: NEW_TASK\nTask name: /root/judge\nSender: /root\nPayload:\n"),
                None,
            ),
            item("encrypted_content", None, Some("gAAAA_SYNTHETIC_2")),
        ]);
        assert_eq!(m.kind, AgentMessageKind::NewTask);
        assert!(m.encrypted);
        assert_eq!(m.body, None);
    }

    #[test]
    fn plaintext_final_answer() {
        let m = parse_received_message(&[item(
            "input_text",
            Some(
                "Message Type: FINAL_ANSWER\nTask name: /root/judge\nSender: /root/judge\nPayload:\nAll good.\n",
            ),
            None,
        )]);
        assert_eq!(m.kind, AgentMessageKind::FinalAnswer);
        assert!(!m.encrypted);
        assert_eq!(m.body.as_deref(), Some("All good."));
    }

    #[test]
    fn spawn_failure_output() {
        assert!(is_failed_spawn_output(
            "collab spawn failed: agent thread limit reached"
        ));
        assert!(!is_failed_spawn_output(r#"{"task_name":"/root/judge"}"#));
    }

    #[test]
    fn truncation_respects_char_boundary() {
        assert_eq!(truncate_bytes("aé", 2), "a");
        assert_eq!(truncate_bytes("abc", 10), "abc");
    }
}
