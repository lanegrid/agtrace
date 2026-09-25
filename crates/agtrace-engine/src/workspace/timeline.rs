//! Compact per-agent timeline rows (the focus pane) and the running-tool tracker.
//!
//! Entries keep only short, display-ready text so that 1000 rows per agent stay
//! cheap; full payloads remain in the log files (re-read via `origin`).

use agtrace_types::{
    AgentEvent, AgentHandle, AgentKind, AgentMessageKind, CompactionTrigger, EventOrigin,
    EventPayload, LifecycleTransition, MessageDirection, SubActionStatus, ToolCallPayload,
    ToolKind, TurnOutcome,
};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Maximum characters kept for any text field of a timeline row.
pub const TIMELINE_TEXT_MAX: usize = 240;

#[derive(Debug, Clone, PartialEq)]
pub struct TimelineEntry {
    pub event_id: Uuid,
    pub ts: DateTime<Utc>,
    pub origin: EventOrigin,
    pub item: TimelineItem,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TimelineItem {
    User {
        text: String,
    },
    Assistant {
        text: String,
    },
    ToolCall {
        name: String,
        kind: ToolKind,
        summary: String,
    },
    /// Only failed results are shown; successful ones just close the running tool.
    ToolError {
        preview: String,
    },
    SubAction {
        summary: String,
        status: SubActionStatus,
        exit_code: Option<i32>,
    },
    Message {
        direction: MessageDirection,
        /// The other side (sender for incoming, first recipient for outgoing), as a label.
        peer: String,
        kind: AgentMessageKind,
        text: Option<String>,
        encrypted: bool,
    },
    Spawn {
        child: String,
        kind: AgentKind,
        agent_type: Option<String>,
        model: Option<String>,
    },
    Lifecycle {
        target: String,
        transition: LifecycleTransition,
        reason: Option<String>,
    },
    Compaction {
        trigger: CompactionTrigger,
        pre_tokens: Option<u64>,
        post_tokens: Option<u64>,
    },
    ModelChange {
        from: Option<String>,
        to: String,
    },
    TurnEnd {
        outcome: TurnOutcome,
        duration_ms: Option<u64>,
    },
    QueueOperation {
        operation: String,
        reason: Option<String>,
        content: Option<String>,
    },
    SlashCommand {
        name: String,
        args: Option<String>,
    },
    Notification {
        kind: Option<String>,
        text: String,
    },
}

/// A tool call that has no result yet.
#[derive(Debug, Clone, PartialEq)]
pub struct RunningTool {
    /// Event id of the ToolCall (matches `ToolResultPayload.tool_call_id`).
    pub call_id: Uuid,
    pub name: String,
    pub kind: ToolKind,
    pub summary: String,
    pub since: DateTime<Utc>,
}

impl RunningTool {
    pub(crate) fn from_call(ev: &AgentEvent, call: &ToolCallPayload) -> Self {
        Self {
            call_id: ev.id,
            name: call.name().to_string(),
            kind: call.kind(),
            summary: tool_summary(call),
            since: ev.timestamp,
        }
    }
}

/// First line of `s`, trimmed and cut to `max` characters (with a trailing `…`).
pub fn one_line(s: &str, max: usize) -> String {
    let line = s
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("");
    if line.chars().count() <= max {
        line.to_string()
    } else {
        let mut out: String = line.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

/// Short human summary of a tool call (command, path, pattern, target...).
pub fn tool_summary(call: &ToolCallPayload) -> String {
    let s = match call {
        ToolCallPayload::Execute { arguments, .. } => arguments
            .command()
            .or(arguments.description.as_deref())
            .unwrap_or_default()
            .to_string(),
        ToolCallPayload::FileRead { arguments, .. } => arguments
            .path()
            .or(arguments.pattern.as_deref())
            .unwrap_or_default()
            .to_string(),
        ToolCallPayload::FileEdit { arguments, .. } => arguments.file_path.clone(),
        ToolCallPayload::FileWrite { arguments, .. } => arguments.file_path.clone(),
        ToolCallPayload::Search { arguments, .. } => {
            arguments.pattern().unwrap_or_default().to_string()
        }
        ToolCallPayload::Mcp { arguments, .. } => match (&arguments.server, &arguments.tool) {
            (Some(s), Some(t)) => format!("{s}/{t}"),
            (None, Some(t)) => t.clone(),
            (Some(s), None) => s.clone(),
            (None, None) => String::new(),
        },
        ToolCallPayload::Agent { arguments, .. } => {
            let op = format!("{:?}", arguments.op).to_lowercase();
            let who = arguments
                .target
                .as_deref()
                .or(arguments.name.as_deref())
                .or(arguments.agent_type.as_deref());
            let text = arguments
                .summary
                .as_deref()
                .or(arguments.message_preview.as_deref());
            match (who, text) {
                (Some(w), Some(t)) => format!("{op} {w}: {t}"),
                (Some(w), None) => format!("{op} {w}"),
                (None, Some(t)) => format!("{op}: {t}"),
                (None, None) => op,
            }
        }
        ToolCallPayload::Generic { .. } => String::new(),
    };
    one_line(&s, TIMELINE_TEXT_MAX)
}

/// Fallback display label of a handle that is not (yet) resolved to an agent.
pub fn handle_label(handle: &AgentHandle) -> String {
    match handle {
        AgentHandle::Id(id) => id.as_str().to_string(),
        AgentHandle::TeamMember { name, .. } => name.clone(),
        AgentHandle::Path(p) => p.clone(),
        AgentHandle::NativeAgentId(a) => a.clone(),
        AgentHandle::User => "user".to_string(),
        AgentHandle::Unknown(s) => s.clone(),
    }
}

/// Timeline row for an event of the agent's own log, or `None` for events that are
/// not shown (usage, hints, attributes, successful tool results, reasoning).
///
/// `label` renders handles (resolved agent label if known).
pub(crate) fn timeline_item(
    ev: &AgentEvent,
    label: &dyn Fn(&AgentHandle) -> String,
) -> Option<TimelineItem> {
    let t = |s: &str| one_line(s, TIMELINE_TEXT_MAX);
    let item = match &ev.payload {
        EventPayload::User(u) => TimelineItem::User { text: t(&u.text) },
        EventPayload::Message(m) => TimelineItem::Assistant { text: t(&m.text) },
        EventPayload::ToolCall(c) => TimelineItem::ToolCall {
            name: c.name().to_string(),
            kind: c.kind(),
            summary: tool_summary(c),
        },
        EventPayload::ToolResult(r) if r.is_error => TimelineItem::ToolError {
            preview: t(&r.output),
        },
        EventPayload::ToolSubAction(s) => {
            let summary = tool_summary(&s.call);
            TimelineItem::SubAction {
                summary: if summary.is_empty() {
                    s.call.name().to_string()
                } else {
                    summary
                },
                status: s.status,
                exit_code: s.exit_code,
            }
        }
        EventPayload::AgentMessage(m) => {
            let peer = match m.direction {
                MessageDirection::Incoming => label(&m.from),
                MessageDirection::Outgoing => m.to.iter().map(label).collect::<Vec<_>>().join(", "),
            };
            TimelineItem::Message {
                direction: m.direction,
                peer,
                kind: m.kind.clone(),
                text: m.body.as_deref().or(m.summary.as_deref()).map(&t),
                encrypted: m.encrypted,
            }
        }
        EventPayload::AgentSpawn(s) => TimelineItem::Spawn {
            child: s.name.clone().unwrap_or_else(|| label(&s.child)),
            kind: s.kind,
            agent_type: s.agent_type.clone(),
            model: s
                .resolved_model
                .clone()
                .or_else(|| s.requested_model.clone()),
        },
        EventPayload::AgentLifecycle(l) => TimelineItem::Lifecycle {
            target: label(&l.target),
            transition: l.transition,
            reason: l.reason.as_deref().map(&t),
        },
        EventPayload::Compaction(c) => TimelineItem::Compaction {
            trigger: c.trigger,
            pre_tokens: c.pre_tokens,
            post_tokens: c.post_tokens,
        },
        EventPayload::ModelChange(m) => TimelineItem::ModelChange {
            from: m.from.clone(),
            to: m.to.clone(),
        },
        EventPayload::TurnEnd(e) => TimelineItem::TurnEnd {
            outcome: e.outcome.clone(),
            duration_ms: e.duration_ms,
        },
        EventPayload::QueueOperation(q) => TimelineItem::QueueOperation {
            operation: q.operation.clone(),
            reason: q.reason.clone(),
            content: q.content.as_deref().map(&t),
        },
        EventPayload::SlashCommand(s) => TimelineItem::SlashCommand {
            name: s.name.clone(),
            args: s.args.as_deref().map(&t),
        },
        EventPayload::Notification(n) => TimelineItem::Notification {
            kind: n.kind.clone(),
            text: t(&n.text),
        },
        EventPayload::ToolResult(_)
        | EventPayload::Reasoning(_)
        | EventPayload::TokenUsage(_)
        | EventPayload::ContextWindowHint(_)
        | EventPayload::AgentAttribute(_)
        // Shown in the detail's plan (the tool calls behind it are on the timeline).
        | EventPayload::Plan(_) => return None,
    };
    Some(item)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_line_takes_first_non_empty_line_and_cuts() {
        assert_eq!(one_line("\n  hello \nworld", 10), "hello");
        assert_eq!(one_line("abcdef", 4), "abc…");
        assert_eq!(one_line("日本語テキスト", 3), "日本…");
        assert_eq!(one_line("", 3), "");
    }
}
