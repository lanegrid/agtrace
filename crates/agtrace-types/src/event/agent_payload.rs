//! Payloads describing multi-agent structure, turn boundaries and context evidence.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::agent::{AgentId, AgentKind};
use crate::tool::ToolCallPayload;

/// How another agent is referred to from inside a log. Resolved to an [`AgentId`] by the graph.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "by", content = "value", rename_all = "snake_case")]
pub enum AgentHandle {
    /// Fully resolved already (Codex agent_thread_id, Claude subagent in the same session).
    Id(AgentId),
    /// Claude teammate name ("audit-A", "team-lead") within a team.
    TeamMember {
        team: Option<String>,
        name: String,
    },
    /// Codex agent_path ("/root", "/root/judge") within the same root.
    Path(String),
    /// Claude subagent agentId or task id seen in notifications.
    NativeAgentId(String),
    /// The human / user.
    User,
    Unknown(String),
}

/// A child agent was spawned from this (parent) log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSpawnPayload {
    pub child: AgentHandle,
    /// Teammate | Subagent | Fork | CodexThread
    pub kind: AgentKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_type: Option<String>,
    /// Model requested by the spawn call ("opus", "gpt-5.6-sol").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_model: Option<String>,
    /// Model the provider resolved ("claude-opus-5-5[1m]").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_model: Option<String>,
    /// Reasoning effort requested by the spawn call (Codex `reasoning_effort`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_effort: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Provider call id in THIS (parent) log.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spawn_call_id: Option<String>,
    /// The ToolCall event that spawned the child, if seen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<Uuid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleTransition {
    Running,
    Idle,
    Interrupted,
    Completed,
    Failed,
    Killed,
    /// All background agents of the session were killed (target = Unknown).
    AllBackgroundKilled,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AgentRunUsage {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_uses: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentLifecyclePayload {
    pub target: AgentHandle,
    pub transition: LifecycleTransition,
    /// idleReason, failureReason, error message
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<AgentRunUsage>,
}

/// Direction relative to the agent whose log contains the message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageDirection {
    Incoming,
    Outgoing,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentMessageKind {
    Message,
    NewTask,
    FinalAnswer,
    Handback,
    TaskNotification,
    IdleNotification,
    Interrupt,
    Peer,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentMessagePayload {
    pub direction: MessageDirection,
    pub from: AgentHandle,
    pub to: Vec<AgentHandle>,
    pub kind: AgentMessageKind,
    /// None when encrypted; truncated by the decoder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(default)]
    pub encrypted: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub triggers_turn: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_message_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompactionTrigger {
    Auto,
    Manual,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompactionPayload {
    pub trigger: CompactionTrigger,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pre_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_number: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum TurnOutcome {
    Completed,
    Interrupted,
    Failed { error: Option<String> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnEndPayload {
    pub outcome: TurnOutcome,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// Codex turn_id
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<String>,
    /// Claude turn_duration.pendingBackgroundAgentCount
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_background_agents: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelChangeSource {
    TurnContext,
    ThreadSettings,
    AssistantMessage,
    LocalCommand,
    Attachment,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelChangePayload {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    pub to: String,
    pub source: ModelChangeSource,
}

/// In-log evidence about the context window. Consumed only by the context resolver.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ContextWindowHintPayload {
    /// Explicit number (Codex task_started / token_count.info.model_context_window).
    Explicit { tokens: u64, model: Option<String> },
    /// Model id carrying an extended-context marker ("[1m]") or "(1M context)".
    ExtendedMarker {
        model: String,
        tokens: u64,
        evidence: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentAttributeKey {
    Title,
    AgentName,
    AgentType,
    TeamName,
    PermissionMode,
    ContinuedIn,
    Cwd,
    /// Runtime (process) session id that wrote records of this transcript, when it
    /// differs from the transcript id (Claude resume / bg daemon respawn). Unlike the
    /// other keys this one accumulates: every distinct value is an alias of the agent
    /// (e.g. a team config's `leadSessionId` may name it).
    RuntimeSessionId,
    /// Reasoning effort the agent currently runs with ("low", "medium", "high"; Claude
    /// record `effort`, Codex `turn_context.effort` / thread settings).
    Effort,
}

/// Agent attribute; upserted by key (latest wins), except
/// [`AgentAttributeKey::RuntimeSessionId`] (one event per distinct value).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentAttributePayload {
    pub key: AgentAttributeKey,
    pub value: String,
}

// ---------------------------------------------------------------- plan

/// Status of a task-list item.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanItemStatus {
    Pending,
    InProgress,
    Completed,
    /// Removed from the list (Claude `TaskUpdate{status:"deleted"}`).
    Deleted,
    Other(String),
}

impl PlanItemStatus {
    /// Provider status string (`pending`, `in_progress`, `completed`, `deleted`).
    pub fn parse(s: &str) -> Self {
        match s {
            "pending" | "todo" | "not_started" => PlanItemStatus::Pending,
            "in_progress" | "in-progress" | "active" => PlanItemStatus::InProgress,
            "completed" | "done" | "complete" => PlanItemStatus::Completed,
            "deleted" | "removed" => PlanItemStatus::Deleted,
            other => PlanItemStatus::Other(other.to_string()),
        }
    }
}

/// One item of a task list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanItem {
    /// Provider task id (Claude TaskCreate `task.id`); None for list-only tools.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub subject: String,
    /// Present-continuous form shown while the item is in progress ("Running tests").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_form: Option<String>,
    pub status: PlanItemStatus,
}

/// What the agent plans to do: its task list, its plan text or its goal.
///
/// Task-list operations carry `team` when the list is shared by an Agent Team
/// (Claude teammates and their lead work on one list; ids are unique per team).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum PlanPayload {
    /// A task was added (Claude `TaskCreate`, id from its result).
    TaskCreated {
        item: PlanItem,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        team: Option<String>,
    },
    /// A task changed (Claude `TaskUpdate`); absent fields are unchanged.
    TaskUpdated {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        status: Option<PlanItemStatus>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        subject: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        active_form: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        team: Option<String>,
    },
    /// The whole list replaced (legacy Claude `TodoWrite`).
    Items { items: Vec<PlanItem> },
    /// Free-form plan (Codex plan-mode `Plan` item, markdown).
    Text { text: String },
    /// The thread's goal changed (Codex `thread_goal_updated`).
    Goal {
        objective: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        status: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubActionStatus {
    Completed,
    Failed,
}

/// Sub-action executed inside a tool call (Codex exec sub-items).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSubActionPayload {
    /// The enclosing ToolCall, when correlated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_tool_call_id: Option<Uuid>,
    pub call: ToolCallPayload,
    pub status: SubActionStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    /// First 2 KiB of the output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_preview: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EventPayload;

    #[test]
    fn new_payloads_roundtrip() {
        let payloads = vec![
            EventPayload::AgentSpawn(AgentSpawnPayload {
                child: AgentHandle::TeamMember {
                    team: Some("t".into()),
                    name: "audit-A".into(),
                },
                kind: AgentKind::Teammate,
                name: Some("audit-A".into()),
                agent_type: None,
                requested_model: Some("opus".into()),
                resolved_model: None,
                requested_effort: Some("medium".into()),
                description: None,
                spawn_call_id: Some("call-1".into()),
                tool_call_id: None,
            }),
            EventPayload::AgentLifecycle(AgentLifecyclePayload {
                target: AgentHandle::Id(AgentId::codex_thread("t1")),
                transition: LifecycleTransition::Completed,
                reason: None,
                usage: Some(AgentRunUsage {
                    total_tokens: Some(10),
                    ..Default::default()
                }),
            }),
            EventPayload::AgentMessage(AgentMessagePayload {
                direction: MessageDirection::Outgoing,
                from: AgentHandle::Path("/root".into()),
                to: vec![AgentHandle::Path("/root/judge".into())],
                kind: AgentMessageKind::Other("custom".into()),
                body: None,
                encrypted: true,
                summary: None,
                triggers_turn: Some(true),
                provider_message_id: None,
            }),
            EventPayload::Compaction(CompactionPayload {
                trigger: CompactionTrigger::Auto,
                pre_tokens: Some(1000),
                post_tokens: None,
                window_number: Some(2),
                duration_ms: None,
            }),
            EventPayload::TurnEnd(TurnEndPayload {
                outcome: TurnOutcome::Failed {
                    error: Some("boom".into()),
                },
                duration_ms: Some(5),
                turn_id: None,
                pending_background_agents: None,
            }),
            EventPayload::ModelChange(ModelChangePayload {
                from: None,
                to: "m".into(),
                source: ModelChangeSource::TurnContext,
            }),
            EventPayload::ContextWindowHint(ContextWindowHintPayload::Explicit {
                tokens: 258_400,
                model: None,
            }),
            EventPayload::AgentAttribute(AgentAttributePayload {
                key: AgentAttributeKey::Title,
                value: "x".into(),
            }),
            EventPayload::Plan(PlanPayload::TaskCreated {
                item: PlanItem {
                    id: Some("1".into()),
                    subject: "Run tests".into(),
                    active_form: Some("Running tests".into()),
                    status: PlanItemStatus::Pending,
                },
                description: None,
                team: Some("t".into()),
            }),
            EventPayload::Plan(PlanPayload::TaskUpdated {
                id: "1".into(),
                status: Some(PlanItemStatus::Other("blocked".into())),
                subject: None,
                active_form: None,
                team: None,
            }),
            EventPayload::Plan(PlanPayload::Goal {
                objective: "ship".into(),
                status: Some("active".into()),
            }),
        ];
        for p in payloads {
            let json = serde_json::to_value(&p).unwrap();
            let back: EventPayload = serde_json::from_value(json.clone()).unwrap();
            assert_eq!(serde_json::to_value(&back).unwrap(), json);
        }
    }
}
