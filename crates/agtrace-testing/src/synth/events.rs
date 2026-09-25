//! `AgentEvent` builders.

use agtrace_types::{
    AgentAttributeKey, AgentAttributePayload, AgentEvent, AgentHandle, AgentId, AgentKind,
    AgentLifecyclePayload, AgentMessageKind, AgentMessagePayload, AgentSpawnPayload,
    CompactionPayload, CompactionTrigger, ContextWindowHintPayload, EventOrigin, EventPayload,
    ExecuteArgs, LifecycleTransition, MessageDirection, MessagePayload, ModelChangePayload,
    ModelChangeSource, NotificationPayload, TokenInput, TokenOutput, TokenUsagePayload,
    ToolCallPayload, ToolResultPayload, TurnEndPayload, TurnOutcome, UserPayload,
};
use chrono::{DateTime, Duration, TimeZone, Utc};
use uuid::Uuid;

/// `2026-09-20T12:00:00Z + secs`: the fixed synthetic clock.
pub fn ts(secs: i64) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, 20, 12, 0, 0).unwrap() + Duration::seconds(secs)
}

/// Shorthands for [`AgentHandle`]s.
pub mod handle {
    use agtrace_types::{AgentHandle, AgentId};

    pub fn id(id: &AgentId) -> AgentHandle {
        AgentHandle::Id(id.clone())
    }
    pub fn member(team: Option<&str>, name: &str) -> AgentHandle {
        AgentHandle::TeamMember {
            team: team.map(str::to_string),
            name: name.to_string(),
        }
    }
    pub fn path(p: &str) -> AgentHandle {
        AgentHandle::Path(p.to_string())
    }
    pub fn native(aid: &str) -> AgentHandle {
        AgentHandle::NativeAgentId(aid.to_string())
    }
}

/// Emits the events of one agent's log in file order: each call is one line
/// (`origin.line` increments), ids are deterministic, `parent_id` chains the events.
///
/// The clock starts at [`ts`]`(0)`; set it with [`EventLog::at`].
#[derive(Debug, Clone)]
pub struct EventLog {
    agent: AgentId,
    session: Uuid,
    line: u64,
    clock: DateTime<Utc>,
    tip: Option<Uuid>,
}

impl EventLog {
    pub fn new(agent: &AgentId) -> Self {
        Self {
            agent: agent.clone(),
            session: Uuid::new_v5(&Uuid::NAMESPACE_OID, agent.native_session_id().as_bytes()),
            line: 0,
            clock: ts(0),
            tip: None,
        }
    }

    pub fn agent(&self) -> &AgentId {
        &self.agent
    }

    /// Set the clock to [`ts`]`(secs)` for the following events.
    pub fn at(&mut self, secs: i64) -> &mut Self {
        self.clock = ts(secs);
        self
    }

    /// Emit an arbitrary payload as the next line.
    pub fn push(&mut self, payload: EventPayload) -> AgentEvent {
        let id = Uuid::new_v5(
            &self.session,
            format!("{}:{}", self.agent.as_str(), self.line).as_bytes(),
        );
        let ev = AgentEvent {
            id,
            session_id: self.session,
            agent: self.agent.clone(),
            parent_id: self.tip,
            timestamp: self.clock,
            origin: EventOrigin::new(self.line, self.line * 100, 0),
            payload,
        };
        self.line += 1;
        self.tip = Some(id);
        ev
    }

    pub fn user(&mut self, text: &str) -> AgentEvent {
        self.push(EventPayload::User(UserPayload {
            text: text.to_string(),
        }))
    }

    pub fn assistant(&mut self, text: &str) -> AgentEvent {
        self.push(EventPayload::Message(MessagePayload::new(text)))
    }

    /// `Bash`-like tool call; pass the returned event's `id` to [`EventLog::tool_result`].
    pub fn bash(&mut self, command: &str) -> AgentEvent {
        self.push(EventPayload::ToolCall(ToolCallPayload::Execute {
            name: "Bash".to_string(),
            arguments: ExecuteArgs {
                command: Some(command.to_string()),
                description: None,
                timeout: None,
                extra: serde_json::json!({}),
            },
            provider_call_id: Some(format!("toolu_synthetic_{}", self.line)),
        }))
    }

    pub fn tool_call(&mut self, call: ToolCallPayload) -> AgentEvent {
        self.push(EventPayload::ToolCall(call))
    }

    pub fn tool_result(&mut self, call_id: Uuid, output: &str, is_error: bool) -> AgentEvent {
        self.push(EventPayload::ToolResult(ToolResultPayload {
            output: output.to_string(),
            tool_call_id: call_id,
            is_error,
        }))
    }

    pub fn turn_end(&mut self) -> AgentEvent {
        self.turn_end_with(TurnOutcome::Completed)
    }

    pub fn turn_end_with(&mut self, outcome: TurnOutcome) -> AgentEvent {
        self.push(EventPayload::TurnEnd(TurnEndPayload {
            outcome,
            duration_ms: None,
            turn_id: None,
            pending_background_agents: None,
        }))
    }

    /// Usage whose context occupancy (`input.total()`) is `context_tokens`.
    pub fn usage(&mut self, context_tokens: u64, model: Option<&str>) -> AgentEvent {
        self.push(EventPayload::TokenUsage(
            TokenUsagePayload::new(
                TokenInput::new(context_tokens, 0, 0),
                TokenOutput::new(10, 0, 0),
            )
            .with_model(model.map(str::to_string)),
        ))
    }

    pub fn model_change(&mut self, from: Option<&str>, to: &str) -> AgentEvent {
        self.push(EventPayload::ModelChange(ModelChangePayload {
            from: from.map(str::to_string),
            to: to.to_string(),
            source: ModelChangeSource::TurnContext,
        }))
    }

    pub fn window_hint(&mut self, tokens: u64) -> AgentEvent {
        self.push(EventPayload::ContextWindowHint(
            ContextWindowHintPayload::Explicit {
                tokens,
                model: None,
            },
        ))
    }

    pub fn compaction(&mut self, pre: Option<u64>, post: Option<u64>) -> AgentEvent {
        self.push(EventPayload::Compaction(CompactionPayload {
            trigger: CompactionTrigger::Auto,
            pre_tokens: pre,
            post_tokens: post,
            window_number: None,
            duration_ms: None,
        }))
    }

    pub fn attribute(&mut self, key: AgentAttributeKey, value: &str) -> AgentEvent {
        self.push(EventPayload::AgentAttribute(AgentAttributePayload {
            key,
            value: value.to_string(),
        }))
    }

    pub fn notification(&mut self, kind: &str, text: &str) -> AgentEvent {
        self.push(EventPayload::Notification(NotificationPayload {
            text: text.to_string(),
            level: None,
            kind: Some(kind.to_string()),
        }))
    }

    /// `AgentSpawn` of `child` (kind, optional name).
    pub fn spawn(&mut self, child: AgentHandle, kind: AgentKind, name: Option<&str>) -> AgentEvent {
        self.spawn_with(AgentSpawnPayload {
            child,
            kind,
            name: name.map(str::to_string),
            agent_type: None,
            requested_model: None,
            resolved_model: None,
            requested_effort: None,
            description: None,
            spawn_call_id: Some(format!("call_synthetic_{}", self.line)),
            tool_call_id: None,
        })
    }

    pub fn spawn_with(&mut self, payload: AgentSpawnPayload) -> AgentEvent {
        self.push(EventPayload::AgentSpawn(payload))
    }

    pub fn lifecycle(
        &mut self,
        target: AgentHandle,
        transition: LifecycleTransition,
    ) -> AgentEvent {
        self.push(EventPayload::AgentLifecycle(AgentLifecyclePayload {
            target,
            transition,
            reason: None,
            usage: None,
        }))
    }

    /// Plaintext inter-agent message.
    pub fn message(
        &mut self,
        direction: MessageDirection,
        from: AgentHandle,
        to: Vec<AgentHandle>,
        kind: AgentMessageKind,
        body: &str,
    ) -> AgentEvent {
        self.message_with(AgentMessagePayload {
            direction,
            from,
            to,
            kind,
            body: Some(body.to_string()),
            encrypted: false,
            summary: None,
            triggers_turn: None,
            provider_message_id: None,
        })
    }

    /// Encrypted inter-agent message (no body), as Codex writes them.
    pub fn encrypted_message(
        &mut self,
        direction: MessageDirection,
        from: AgentHandle,
        to: Vec<AgentHandle>,
        kind: AgentMessageKind,
    ) -> AgentEvent {
        self.message_with(AgentMessagePayload {
            direction,
            from,
            to,
            kind,
            body: None,
            encrypted: true,
            summary: None,
            triggers_turn: None,
            provider_message_id: None,
        })
    }

    pub fn message_with(&mut self, payload: AgentMessagePayload) -> AgentEvent {
        self.push(EventPayload::AgentMessage(payload))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_chain_in_file_order() {
        let id = AgentId::claude_session("s1");
        let mut log = EventLog::new(&id);
        let a = log.user("hi");
        let b = log.at(5).assistant("hello");
        assert_eq!(a.origin.line, 0);
        assert_eq!(b.origin.line, 1);
        assert_eq!(b.parent_id, Some(a.id));
        assert_eq!(b.timestamp, ts(5));
        assert_ne!(a.id, b.id);
        // Deterministic ids.
        let mut again = EventLog::new(&id);
        assert_eq!(again.user("other").id, a.id);
    }
}
