//! Line-oriented Claude Code (≥ 2.1.24x) decoder (`LogDecoder`).
//!
//! One decoder per agent file. Each line is decoded leniently into a
//! [`ClaudeRecord`] and mapped to events; problems are counted in diagnostics.
//! Cross-line state lives here: tool call map (in the builder), current model,
//! per-`message.id` usage (dedupe + upsert), pending agent-tool calls,
//! notification dedupe, latest-wins attributes and seen context markers.

use std::collections::{HashMap, HashSet, VecDeque};

use agtrace_types::*;
use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

use super::content::{AssistantContent, AssistantMessage, Usage, UserContent};
use super::mapper::normalize_claude_tool_call;
use super::records::*;
use super::tags::{
    TaskNotification, find_tags, is_subagent_id, set_model_output, slash_command, starts_with_tag,
    strip_delivery_preamble, truncate_bytes,
};
use crate::builder::{EventBuilder, SemanticSuffix};
use crate::lenient::{RawLine, decode_typed, flatten_tool_output, kind_label_of};
use crate::provider::{DecodeOptions, FileHeader, LogDecoder};

/// Placeholder for thinking blocks persisted without plaintext (signature only).
const REDACTED_THINKING_MARKER: &str = "[thinking redacted]";
/// Maximum size of message bodies kept in `AgentMessage` events.
const BODY_MAX_BYTES: usize = 4096;
/// Implied size of an extended ("1M") context window.
const EXTENDED_CONTEXT_TOKENS: u64 = 1_000_000;
/// How many recent `message.id`s are remembered for usage dedupe / redaction markers.
/// Split records of one message are (nearly) contiguous, so a small window suffices.
const RECENT_MESSAGES: usize = 256;

/// Small insertion-ordered map that forgets the oldest keys beyond a capacity.
struct RecentMap<V> {
    map: HashMap<String, V>,
    order: VecDeque<String>,
}

impl<V> RecentMap<V> {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    fn get(&self, key: &str) -> Option<&V> {
        self.map.get(key)
    }

    fn insert(&mut self, key: String, value: V) {
        if self.map.insert(key.clone(), value).is_none() {
            self.order.push_back(key);
            if self.order.len() > RECENT_MESSAGES
                && let Some(old) = self.order.pop_front()
            {
                self.map.remove(&old);
            }
        }
    }
}

/// An agent-management tool call awaiting its result (`toolUseResult` carries the outcome).
struct PendingAgentCall {
    name: String,
    input: Value,
    event_id: Uuid,
}

impl PendingAgentCall {
    fn input_str(&self, key: &str) -> Option<String> {
        str_of(&self.input, key)
    }
}

fn str_of(v: &Value, key: &str) -> Option<String> {
    v.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// Model ids that are not real models (synthetic error / interrupt messages).
fn is_real_model(model: &str) -> bool {
    !model.is_empty() && !model.starts_with('<')
}

/// Model id without a context-variant suffix (`claude-opus-5-5[1m]` -> `claude-opus-5-5`).
/// `message.model` never carries the suffix; attachments and cost keys sometimes do.
fn base_model_id(model: &str) -> &str {
    match model.find('[') {
        Some(i) if model.ends_with(']') => &model[..i],
        _ => model,
    }
}

/// "Opus 5.5 (1M context)" -> "claude-opus-5-5" (display name to model id).
fn model_id_from_display_name(name: &str) -> Option<String> {
    let base = name.split('(').next().unwrap_or("").trim();
    if base.is_empty() {
        return None;
    }
    let slug: String = base
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    Some(format!("claude-{slug}"))
}

fn is_extended_display_name(name: &str) -> bool {
    name.contains("(1M context)")
}

/// Label of an ignored attachment (`attachment/<type>`), read from the raw line
/// without a second full parse (the attachment object starts with its `type`).
fn attachment_label(text: &str) -> String {
    const KEY: &str = r#""attachment":{"type":""#;
    text.find(KEY)
        .map(|i| &text[i + KEY.len()..])
        .and_then(|rest| rest.find('"').map(|end| &rest[..end]))
        .map(|ty| format!("attachment/{ty}"))
        .unwrap_or_else(|| "attachment".to_string())
}

fn body(text: &str) -> Option<String> {
    let t = text.trim();
    (!t.is_empty()).then(|| truncate_bytes(t, BODY_MAX_BYTES))
}

/// Per-record emission context.
struct Rec {
    base: String,
    ts: DateTime<Utc>,
    agent: AgentId,
}

pub struct ClaudeDecoder {
    builder: EventBuilder,
    /// Agent owning the file (from the file header).
    agent: AgentId,
    agent_kind: AgentKind,
    /// Team of this agent (teammate) or of the teammates it spawned (lead).
    team: Option<String>,
    diagnostics: ParseDiagnostics,
    line: u64,
    last_timestamp: Option<DateTime<Utc>>,
    model: Option<String>,
    /// Last emitted usage per message.id (dedupe + upsert).
    usage: RecentMap<TokenUsagePayload>,
    /// message.ids that already produced a redacted-thinking marker.
    redacted: RecentMap<()>,
    /// Agent tool calls (Agent / SendMessage / TaskStop) by provider call id.
    agent_calls: HashMap<String, PendingAgentCall>,
    /// (task-id | sender, status) of notifications already emitted.
    seen_notifications: HashSet<(String, String)>,
    /// Latest value per attribute (emit only on change).
    attributes: HashMap<AgentAttributeKey, String>,
    /// Models already reported as extended-context.
    context_markers: HashSet<String>,
    seen_user_record: bool,
    /// Transcript session id of the file (records' `sessionId`).
    session_id: String,
    /// Runtime session ids already reported as aliases.
    runtime_ids: HashSet<String>,
}

impl ClaudeDecoder {
    pub fn new(header: &FileHeader, opts: DecodeOptions) -> Self {
        let session_id = header.agent.native_session_id.clone();
        let session_uuid = Uuid::new_v5(&Uuid::NAMESPACE_OID, session_id.as_bytes());
        Self {
            builder: EventBuilder::new(session_uuid),
            agent: header.agent.id.clone(),
            agent_kind: header.agent.kind,
            team: header.agent.team.clone(),
            diagnostics: ParseDiagnostics::default(),
            line: 0,
            last_timestamp: opts.fallback_timestamp,
            model: None,
            usage: RecentMap::new(),
            redacted: RecentMap::new(),
            agent_calls: HashMap::new(),
            seen_notifications: HashSet::new(),
            attributes: HashMap::new(),
            context_markers: HashSet::new(),
            seen_user_record: false,
            session_id,
            runtime_ids: HashSet::new(),
        }
    }

    // ---- helpers ----

    /// Parse a record timestamp; missing / unparsable timestamps inherit the last seen one.
    fn ts(&mut self, raw: Option<&str>) -> DateTime<Utc> {
        if let Some(dt) = raw.and_then(|t| DateTime::parse_from_rfc3339(t).ok()) {
            let dt = dt.with_timezone(&Utc);
            self.last_timestamp = Some(dt);
            return dt;
        }
        self.last_timestamp.unwrap_or(DateTime::<Utc>::UNIX_EPOCH)
    }

    /// Base id of a record: its uuid, else a per-file line key.
    fn base(&self, uuid: Option<&str>) -> String {
        match uuid.filter(|u| !u.is_empty()) {
            Some(u) => u.to_string(),
            None => format!("{}#L{}", self.agent, self.line),
        }
    }

    fn rec(&mut self, env: &Envelope) -> Rec {
        if self.team.is_none() {
            self.team.clone_from(&env.team_name);
        }
        Rec {
            base: self.base(env.uuid.as_deref()),
            ts: self.ts(env.timestamp.as_deref()),
            agent: self.agent.clone(),
        }
    }

    /// Context for records without an envelope (state records, queue ops, ...).
    fn bare_rec(&mut self, timestamp: Option<&str>) -> Rec {
        Rec {
            base: self.base(None),
            ts: self.ts(timestamp),
            agent: self.agent.clone(),
        }
    }

    fn push(
        &mut self,
        out: &mut Vec<AgentEvent>,
        r: &Rec,
        base: &str,
        suffix: SemanticSuffix,
        payload: EventPayload,
    ) -> Uuid {
        self.builder
            .build_and_push(out, base, suffix, r.ts, payload, &r.agent)
    }

    fn me(&self) -> AgentHandle {
        AgentHandle::Id(self.agent.clone())
    }

    fn member(&self, name: impl Into<String>) -> AgentHandle {
        AgentHandle::TeamMember {
            team: self.team.clone(),
            name: name.into(),
        }
    }

    fn set_model(
        &mut self,
        out: &mut Vec<AgentEvent>,
        r: &Rec,
        base: &str,
        model: &str,
        source: ModelChangeSource,
    ) {
        let model = base_model_id(model);
        if self.model.as_deref() == Some(model) {
            return;
        }
        let from = self.model.replace(model.to_string());
        self.push(
            out,
            r,
            base,
            SemanticSuffix::ModelChange,
            EventPayload::ModelChange(ModelChangePayload {
                from,
                to: model.to_string(),
                source,
            }),
        );
    }

    /// ContextWindowHint::ExtendedMarker, once per (subject, model) per file. `model` is
    /// kept as observed (it may carry a `[1m]` suffix). `subject` separates evidence
    /// about this agent ("self") from evidence about a spawned child ("spawn").
    fn extended_marker(
        &mut self,
        out: &mut Vec<AgentEvent>,
        r: &Rec,
        subject: &str,
        model: &str,
        evidence: String,
    ) {
        // Suffix variants of the same id count once.
        let key = format!("{subject}:{}", base_model_id(model));
        if !self.context_markers.insert(key.clone()) {
            return;
        }
        let base = format!("{}#ctx-{}", self.agent, key);
        self.push(
            out,
            r,
            &base,
            SemanticSuffix::ContextWindowHint,
            EventPayload::ContextWindowHint(ContextWindowHintPayload::ExtendedMarker {
                model: model.to_string(),
                tokens: EXTENDED_CONTEXT_TOKENS,
                evidence,
            }),
        );
    }

    /// AgentAttribute, emitted only when the value changes. The id is stable per key
    /// (upsert: the latest value replaces earlier ones).
    fn attribute(
        &mut self,
        out: &mut Vec<AgentEvent>,
        r: &Rec,
        key: AgentAttributeKey,
        value: Option<String>,
    ) {
        let Some(value) = value.filter(|v| !v.is_empty()) else {
            return;
        };
        if self.attributes.get(&key) == Some(&value) {
            return;
        }
        self.attributes.insert(key, value.clone());
        let base = format!("{}#attr-{:?}", self.agent, key);
        self.push(
            out,
            r,
            &base,
            SemanticSuffix::AgentAttribute,
            EventPayload::AgentAttribute(AgentAttributePayload { key, value }),
        );
    }

    /// `AgentAttribute(RuntimeSessionId)` once per distinct runtime `session_id` that
    /// differs from the transcript id (resumed / respawned processes write into the
    /// same transcript under a new runtime id; team configs name the runtime id).
    fn runtime_alias(&mut self, out: &mut Vec<AgentEvent>, env: &Envelope) {
        let Some(sid) = env.runtime_session_id.as_deref().filter(|s| !s.is_empty()) else {
            return;
        };
        if sid == self.session_id || !self.runtime_ids.insert(sid.to_string()) {
            return;
        }
        let r = Rec {
            base: String::new(),
            ts: self.ts(env.timestamp.as_deref()),
            agent: self.agent.clone(),
        };
        let base = format!("{}#runtime-{sid}", self.agent);
        self.push(
            out,
            &r,
            &base,
            SemanticSuffix::AgentAttribute,
            EventPayload::AgentAttribute(AgentAttributePayload {
                key: AgentAttributeKey::RuntimeSessionId,
                value: sid.to_string(),
            }),
        );
    }

    fn notification(
        &mut self,
        out: &mut Vec<AgentEvent>,
        r: &Rec,
        base: &str,
        text: String,
        level: &str,
        kind: &str,
    ) {
        self.push(
            out,
            r,
            base,
            SemanticSuffix::Notification,
            EventPayload::Notification(NotificationPayload {
                text,
                level: Some(level.to_string()),
                kind: Some(kind.to_string()),
            }),
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn lifecycle(
        &mut self,
        out: &mut Vec<AgentEvent>,
        r: &Rec,
        base: &str,
        target: AgentHandle,
        transition: LifecycleTransition,
        reason: Option<String>,
        usage: Option<AgentRunUsage>,
    ) {
        self.push(
            out,
            r,
            base,
            SemanticSuffix::AgentLifecycle,
            EventPayload::AgentLifecycle(AgentLifecyclePayload {
                target,
                transition,
                reason,
                usage,
            }),
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn agent_message(
        &mut self,
        out: &mut Vec<AgentEvent>,
        r: &Rec,
        base: &str,
        direction: MessageDirection,
        from: AgentHandle,
        to: Vec<AgentHandle>,
        kind: AgentMessageKind,
        body: Option<String>,
        summary: Option<String>,
        provider_message_id: Option<String>,
    ) {
        self.push(
            out,
            r,
            base,
            SemanticSuffix::AgentMessage,
            EventPayload::AgentMessage(AgentMessagePayload {
                direction,
                from,
                to,
                kind,
                body,
                encrypted: false,
                summary,
                triggers_turn: None,
                provider_message_id,
            }),
        );
    }

    // ---- assistant ----

    fn on_assistant(&mut self, rec: AssistantRecord, out: &mut Vec<AgentEvent>) {
        let r = self.rec(&rec.env);
        let msg: AssistantMessage = rec.message;
        let real_model = msg.model.as_deref().filter(|m| is_real_model(m));
        if let Some(model) = real_model {
            let model = model.to_string();
            self.set_model(
                out,
                &r,
                &r.base.clone(),
                &model,
                ModelChangeSource::AssistantMessage,
            );
        }
        let message_key = msg.id.clone().filter(|id| !id.is_empty());

        for (idx, block) in msg.content.iter().enumerate() {
            let base = format!("{}-content-{}", r.base, idx);
            match block {
                AssistantContent::Thinking {
                    thinking,
                    signature,
                } => {
                    let text = if !thinking.is_empty() {
                        Some(thinking.clone())
                    } else if signature.is_some() {
                        // Redacted thinking: one marker per API message, not per split record.
                        let key = message_key.clone().unwrap_or_else(|| r.base.clone());
                        if self.redacted.get(&key).is_some() {
                            None
                        } else {
                            self.redacted.insert(key, ());
                            Some(REDACTED_THINKING_MARKER.to_string())
                        }
                    } else {
                        None
                    };
                    if let Some(text) = text {
                        self.push(
                            out,
                            &r,
                            &base,
                            SemanticSuffix::Reasoning,
                            EventPayload::Reasoning(ReasoningPayload { text }),
                        );
                    }
                }
                AssistantContent::Text { text } => {
                    self.push(
                        out,
                        &r,
                        &base,
                        SemanticSuffix::Message,
                        EventPayload::Message(MessagePayload::new(text.clone())),
                    );
                }
                AssistantContent::ToolUse { id, name, input } => {
                    let payload =
                        normalize_claude_tool_call(name.clone(), input.clone(), Some(id.clone()));
                    let event_id = self.push(
                        out,
                        &r,
                        &base,
                        SemanticSuffix::ToolCall,
                        EventPayload::ToolCall(payload),
                    );
                    self.builder.register_tool_call(id.clone(), event_id);
                    if matches!(name.as_str(), "Agent" | "SendMessage" | "TaskStop") {
                        self.agent_calls.insert(
                            id.clone(),
                            PendingAgentCall {
                                name: name.clone(),
                                input: input.clone(),
                                event_id,
                            },
                        );
                    }
                }
                AssistantContent::Unknown => {}
            }
        }

        // Synthetic messages (API error / interrupt) carry zero usage: not a request.
        if let Some(usage) = &msg.usage
            && real_model.is_some()
        {
            let payload = usage_payload(usage, msg.stop_reason.is_none())
                .with_model(real_model.map(str::to_string))
                .with_dedupe_key(message_key.clone());
            let key = message_key.unwrap_or_else(|| r.base.clone());
            // One TokenUsage per message.id; re-emitted with the same id when it changes.
            if self.usage.get(&key) != Some(&payload) {
                self.push(
                    out,
                    &r,
                    &key,
                    SemanticSuffix::TokenUsage,
                    EventPayload::TokenUsage(payload.clone()),
                );
                self.usage.insert(key, payload);
            }
        }
    }

    // ---- user ----

    fn on_user(&mut self, rec: UserRecord, out: &mut Vec<AgentEvent>) {
        let r = self.rec(&rec.env);
        let first_user_record = !self.seen_user_record;
        self.seen_user_record = true;
        if rec.is_compact_summary {
            // Compaction is reported by system/compact_boundary.
            return;
        }
        let tool_use_result = rec.tool_use_result.as_ref().filter(|v| v.is_object());
        for (idx, block) in rec.message.content.iter().enumerate() {
            let base = format!("{}-content-{}", r.base, idx);
            match block {
                UserContent::Text { text } => {
                    if !rec.is_meta {
                        self.on_user_text(out, &r, &base, text, first_user_record);
                    }
                }
                UserContent::ToolResult {
                    tool_use_id,
                    content,
                    is_error,
                } => {
                    let tool_call_id = self.builder.get_tool_call_uuid(tool_use_id);
                    if let Some(tool_call_id) = tool_call_id {
                        self.push(
                            out,
                            &r,
                            &base,
                            SemanticSuffix::ToolResult,
                            EventPayload::ToolResult(ToolResultPayload {
                                output: flatten_tool_output(content),
                                tool_call_id,
                                is_error: *is_error,
                            }),
                        );
                    }
                    let call = self.agent_calls.remove(tool_use_id);
                    if let Some(result) = tool_use_result {
                        self.on_agent_tool_result(out, &r, &base, tool_use_id, call, result);
                    }
                }
                UserContent::Unknown => {}
            }
        }
    }

    fn on_user_text(
        &mut self,
        out: &mut Vec<AgentEvent>,
        r: &Rec,
        base: &str,
        text: &str,
        first_user_record: bool,
    ) {
        let t = text.trim();
        if t.is_empty()
            || starts_with_tag(t, "system-reminder")
            || starts_with_tag(t, "local-command-caveat")
            || starts_with_tag(t, "fork-boilerplate")
        {
            return;
        }
        // Agent tags may follow a delivery preamble ("Another Claude session sent a message:").
        let tagged = strip_delivery_preamble(t);
        if starts_with_tag(tagged, "teammate-message") {
            self.teammate_messages(out, r, base, tagged, first_user_record);
        } else if starts_with_tag(tagged, "task-notification") {
            for (i, tag) in find_tags(tagged, "task-notification").iter().enumerate() {
                self.task_notification(out, r, &format!("{base}-tag-{i}"), tag.inner);
            }
        } else if starts_with_tag(tagged, "agent-message") {
            for (i, tag) in find_tags(tagged, "agent-message").iter().enumerate() {
                let from = tag.attr("from").unwrap_or_default();
                self.peer_message(
                    out,
                    r,
                    &format!("{base}-tag-{i}"),
                    &from,
                    tag.inner,
                    false,
                    None,
                );
            }
        } else if let Some((name, args)) = slash_command(t) {
            self.push(
                out,
                r,
                base,
                SemanticSuffix::SlashCommand,
                EventPayload::SlashCommand(SlashCommandPayload { name, args }),
            );
        } else if starts_with_tag(t, "local-command-stdout") {
            self.local_command_output(out, r, base, t);
        } else if t.starts_with("[Request interrupted") {
            self.push(
                out,
                r,
                base,
                SemanticSuffix::TurnEnd,
                EventPayload::TurnEnd(TurnEndPayload {
                    outcome: TurnOutcome::Interrupted,
                    duration_ms: None,
                    turn_id: None,
                    pending_background_agents: None,
                }),
            );
        } else {
            self.push(
                out,
                r,
                base,
                SemanticSuffix::User,
                EventPayload::User(UserPayload {
                    text: text.to_string(),
                }),
            );
        }
    }

    /// `/model` output: ModelChange(LocalCommand) (+ ExtendedMarker for "(1M context)").
    fn local_command_output(&mut self, out: &mut Vec<AgentEvent>, r: &Rec, base: &str, text: &str) {
        let Some(display) = set_model_output(text) else {
            return;
        };
        let model = model_id_from_display_name(&display).unwrap_or_else(|| display.clone());
        self.set_model(out, r, base, &model, ModelChangeSource::LocalCommand);
        if is_extended_display_name(&display) {
            self.extended_marker(
                out,
                r,
                "self",
                &model,
                format!("local command: Set model to {display}"),
            );
        }
    }

    /// `<teammate-message teammate_id=.. [summary=..]>` tags (0..n per record).
    fn teammate_messages(
        &mut self,
        out: &mut Vec<AgentEvent>,
        r: &Rec,
        base: &str,
        text: &str,
        first_user_record: bool,
    ) {
        for (i, tag) in find_tags(text, "teammate-message").iter().enumerate() {
            let base = format!("{base}-tag-{i}");
            let sender = tag.attr("teammate_id").unwrap_or_default();
            let summary = tag.attr("summary");
            let inner = tag.inner.trim();
            let json = inner
                .starts_with('{')
                .then(|| serde_json::from_str::<Value>(inner).ok())
                .flatten()
                .filter(|v| v.get("type").and_then(Value::as_str).is_some());
            match json {
                Some(v) if v.get("type").and_then(Value::as_str) == Some("idle_notification") => {
                    let name = str_of(&v, "from").unwrap_or_else(|| sender.clone());
                    let idle_reason = str_of(&v, "idleReason");
                    let transition = if idle_reason.as_deref() == Some("failed") {
                        LifecycleTransition::Failed
                    } else {
                        LifecycleTransition::Idle
                    };
                    let reason = str_of(&v, "failureReason").or(idle_reason);
                    let target = self.member(name);
                    self.lifecycle(out, r, &base, target, transition, reason, None);
                }
                Some(v) => {
                    let kind = str_of(&v, "type").unwrap_or_default();
                    let (from, me) = (self.member(sender), self.me());
                    self.agent_message(
                        out,
                        r,
                        &base,
                        MessageDirection::Incoming,
                        from,
                        vec![me],
                        AgentMessageKind::Other(kind),
                        body(inner),
                        summary,
                        None,
                    );
                }
                None => {
                    let kind = if first_user_record && self.agent_kind == AgentKind::Teammate {
                        AgentMessageKind::NewTask
                    } else {
                        AgentMessageKind::Message
                    };
                    let (from, me) = (self.member(sender), self.me());
                    self.agent_message(
                        out,
                        r,
                        &base,
                        MessageDirection::Incoming,
                        from,
                        vec![me],
                        kind,
                        body(inner),
                        summary,
                        None,
                    );
                }
            }
        }
    }

    /// `<task-notification>`: agent tasks become lifecycle + message, other tasks
    /// (background bash, monitor, artifacts) a Notification. Deduped across the
    /// queued_command attachment and the user record that deliver the same notification.
    fn task_notification(&mut self, out: &mut Vec<AgentEvent>, r: &Rec, base: &str, inner: &str) {
        let n = TaskNotification::parse(inner);
        let id = n
            .task_id
            .clone()
            .or_else(|| n.tool_use_id.clone())
            .unwrap_or_default();
        let status = n.status.clone().unwrap_or_default();
        // Keyed by the full notification text: the same notification is delivered twice
        // (queued_command + user record), while an agent that stops again produces a new one.
        let key = format!("task:{}", truncate_bytes(inner.trim(), 512));
        if !self.seen_notifications.insert((id.clone(), key)) {
            return;
        }
        if !n.is_agent_task() {
            let text = n
                .summary
                .clone()
                .unwrap_or_else(|| format!("Task {id} {status}").trim().to_string());
            self.notification(out, r, base, text, "info", "task_notification");
            return;
        }
        let target = AgentHandle::NativeAgentId(id);
        let usage = (n.total_tokens.is_some() || n.tool_uses.is_some() || n.duration_ms.is_some())
            .then_some(AgentRunUsage {
                total_tokens: n.total_tokens,
                tool_uses: n.tool_uses,
                duration_ms: n.duration_ms,
            });
        let transition = match status.as_str() {
            "completed" => Some(LifecycleTransition::Completed),
            "failed" => Some(LifecycleTransition::Failed),
            "killed" | "stopped" => Some(LifecycleTransition::Killed),
            _ => None,
        };
        if let Some(transition) = transition {
            let reason = (transition != LifecycleTransition::Completed)
                .then(|| n.summary.clone())
                .flatten();
            self.lifecycle(out, r, base, target.clone(), transition, reason, usage);
        }
        let me = self.me();
        self.agent_message(
            out,
            r,
            base,
            MessageDirection::Incoming,
            target,
            vec![me],
            AgentMessageKind::TaskNotification,
            n.result.as_deref().and_then(body),
            n.summary,
            None,
        );
    }

    /// `<agent-message from=..>`: subagent hand-back (lifecycle Completed + message) or a
    /// peer-session message. Deduped like task notifications.
    #[allow(clippy::too_many_arguments)]
    fn peer_message(
        &mut self,
        out: &mut Vec<AgentEvent>,
        r: &Rec,
        base: &str,
        from: &str,
        text: &str,
        handback_hint: bool,
        usage: Option<AgentRunUsage>,
    ) {
        const HANDBACK_MARKER: &str = "[Subagent hand-back]";
        let text = text.trim();
        let handback = handback_hint || text.starts_with(HANDBACK_MARKER);
        let text = text.strip_prefix(HANDBACK_MARKER).unwrap_or(text).trim();
        let key = format!(
            "{}:{}",
            if handback { "handback" } else { "peer" },
            truncate_bytes(text, 512)
        );
        if !self.seen_notifications.insert((from.to_string(), key)) {
            return;
        }
        let sender = if is_subagent_id(from) {
            AgentHandle::NativeAgentId(from.to_string())
        } else {
            AgentHandle::Unknown(from.to_string())
        };
        let kind = if handback {
            AgentMessageKind::Handback
        } else {
            AgentMessageKind::Peer
        };
        let me = self.me();
        self.agent_message(
            out,
            r,
            base,
            MessageDirection::Incoming,
            sender.clone(),
            vec![me],
            kind,
            body(text),
            None,
            None,
        );
        if handback {
            self.lifecycle(
                out,
                r,
                base,
                sender,
                LifecycleTransition::Completed,
                None,
                usage,
            );
        }
    }

    /// Outcome of an agent-management tool call, from the result record's `toolUseResult`.
    fn on_agent_tool_result(
        &mut self,
        out: &mut Vec<AgentEvent>,
        r: &Rec,
        base: &str,
        tool_use_id: &str,
        call: Option<PendingAgentCall>,
        result: &Value,
    ) {
        let call_str = |key: &str| call.as_ref().and_then(|c| c.input_str(key));
        let tool_call_id = call.as_ref().map(|c| c.event_id);
        match str_of(result, "status").as_deref() {
            Some("teammate_spawned") => {
                let team = str_of(result, "team_name");
                if team.is_some() {
                    self.team.clone_from(&team);
                }
                let name = str_of(result, "name")
                    .or_else(|| {
                        str_of(result, "agent_id")
                            .and_then(|id| id.split('@').next().map(str::to_string))
                    })
                    .or_else(|| call_str("name"))
                    .unwrap_or_default();
                self.push(
                    out,
                    r,
                    base,
                    SemanticSuffix::AgentSpawn,
                    EventPayload::AgentSpawn(AgentSpawnPayload {
                        child: AgentHandle::TeamMember {
                            team,
                            name: name.clone(),
                        },
                        kind: AgentKind::Teammate,
                        name: Some(name).filter(|n| !n.is_empty()),
                        agent_type: str_of(result, "agent_type")
                            .or_else(|| call_str("subagent_type")),
                        requested_model: str_of(result, "model").or_else(|| call_str("model")),
                        resolved_model: None,
                        description: call_str("description"),
                        spawn_call_id: Some(tool_use_id.to_string()),
                        tool_call_id,
                    }),
                );
            }
            Some("async_launched") => {
                let Some(agent_id) = str_of(result, "agentId") else {
                    return;
                };
                let agent_type = call_str("subagent_type");
                let kind = if agent_type.as_deref() == Some("fork") {
                    AgentKind::Fork
                } else {
                    AgentKind::Subagent
                };
                let resolved = str_of(result, "resolvedModel");
                self.push(
                    out,
                    r,
                    base,
                    SemanticSuffix::AgentSpawn,
                    EventPayload::AgentSpawn(AgentSpawnPayload {
                        child: AgentHandle::NativeAgentId(agent_id),
                        kind,
                        name: call_str("name"),
                        agent_type,
                        requested_model: call_str("model"),
                        resolved_model: resolved.clone(),
                        description: str_of(result, "description")
                            .or_else(|| call_str("description")),
                        spawn_call_id: Some(tool_use_id.to_string()),
                        tool_call_id,
                    }),
                );
                if let Some(model) = resolved.filter(|m| m.ends_with("[1m]")) {
                    self.extended_marker(
                        out,
                        r,
                        "spawn",
                        &model,
                        "agent spawn resolvedModel".to_string(),
                    );
                }
            }
            _ => {}
        }

        let Some(call) = call else { return };
        match call.name.as_str() {
            "SendMessage" if result.get("success").and_then(Value::as_bool) == Some(true) => {
                let routing = result.get("routing");
                let target = routing
                    .and_then(|r| str_of(r, "target"))
                    .map(|t| t.trim_start_matches('@').to_string())
                    .or_else(|| call.input_str("to"))
                    .or_else(|| call.input_str("recipient"));
                let Some(target) = target else { return };
                let text = call
                    .input_str("message")
                    .or_else(|| call.input_str("content"))
                    .or_else(|| routing.and_then(|r| str_of(r, "content")));
                let summary = call
                    .input_str("summary")
                    .or_else(|| routing.and_then(|r| str_of(r, "summary")));
                // `to` is a teammate name, or the id of a (background) subagent to resume.
                let to = if is_subagent_id(&target) {
                    AgentHandle::NativeAgentId(target)
                } else {
                    self.member(target)
                };
                let me = self.me();
                self.agent_message(
                    out,
                    r,
                    base,
                    MessageDirection::Outgoing,
                    me,
                    vec![to],
                    AgentMessageKind::Message,
                    text.as_deref().and_then(body),
                    summary,
                    str_of(result, "msg_id"),
                );
            }
            "TaskStop" => {
                let Some(task) = call
                    .input_str("task_id")
                    .or_else(|| str_of(result, "task_id"))
                else {
                    return;
                };
                let target = match str_of(result, "task_type").as_deref() {
                    Some("in_process_teammate") => self.member(task),
                    Some("local_agent") if is_subagent_id(&task) => {
                        AgentHandle::NativeAgentId(task)
                    }
                    _ => return,
                };
                self.lifecycle(
                    out,
                    r,
                    base,
                    target,
                    LifecycleTransition::Killed,
                    None,
                    None,
                );
            }
            _ => {}
        }
    }

    // ---- attachment ----

    fn on_attachment(&mut self, rec: AttachmentRecord, text: &str, out: &mut Vec<AgentEvent>) {
        let r = self.rec(&rec.env);
        let base = r.base.clone();
        match rec.attachment {
            Attachment::QueuedCommand(qc) => self.queued_command(out, &r, &base, qc),
            Attachment::Model(m) => {
                let identity = m.identity.unwrap_or_default();
                let display = identity.marketing_name.unwrap_or_default();
                let model = identity
                    .model_id
                    .filter(|m| is_real_model(m))
                    .or_else(|| model_id_from_display_name(&display));
                if let Some(model) = &model {
                    self.set_model(out, &r, &base, model, ModelChangeSource::Attachment);
                    if is_extended_display_name(&display) {
                        self.extended_marker(
                            out,
                            &r,
                            "self",
                            model,
                            format!("model attachment: {display}"),
                        );
                    }
                }
            }
            Attachment::TeamContext(tc) => {
                if tc.team_name.is_some() {
                    self.team.clone_from(&tc.team_name);
                }
                self.attribute(out, &r, AgentAttributeKey::TeamName, tc.team_name);
            }
            Attachment::PlanModeExit { plan_file_path } => {
                let text = match plan_file_path {
                    Some(path) => format!("Exited plan mode (plan: {path})"),
                    None => "Exited plan mode".to_string(),
                };
                self.notification(out, &r, &base, text, "info", "plan_mode_exit");
            }
            Attachment::Other => self.diagnostics.record_ignored(&attachment_label(text)),
        }
    }

    fn queued_command(
        &mut self,
        out: &mut Vec<AgentEvent>,
        r: &Rec,
        base: &str,
        qc: QueuedCommand,
    ) {
        let prompt = qc
            .prompt
            .as_ref()
            .map(flatten_tool_output)
            .unwrap_or_default();
        let prompt = strip_delivery_preamble(&prompt).to_string();
        let origin = qc.origin.unwrap_or_default();
        let usage = qc.usage.map(|u| AgentRunUsage {
            total_tokens: u.total_tokens,
            tool_uses: u.tool_uses,
            duration_ms: u.duration_ms,
        });

        if origin.kind.as_deref() == Some("peer") || starts_with_tag(&prompt, "agent-message") {
            let tags = find_tags(&prompt, "agent-message");
            if tags.is_empty() {
                let from = origin.from.clone().unwrap_or_default();
                let text = origin.body.clone().unwrap_or_else(|| prompt.clone());
                self.peer_message(out, r, base, &from, &text, origin.handback, usage.clone());
            }
            for (i, tag) in tags.iter().enumerate() {
                let from = tag
                    .attr("from")
                    .or_else(|| origin.from.clone())
                    .unwrap_or_default();
                let base = format!("{base}-tag-{i}");
                self.peer_message(
                    out,
                    r,
                    &base,
                    &from,
                    tag.inner,
                    origin.handback,
                    usage.clone(),
                );
            }
            return;
        }
        if qc.command_mode.as_deref() == Some("task-notification")
            || starts_with_tag(&prompt, "task-notification")
        {
            for (i, tag) in find_tags(&prompt, "task-notification").iter().enumerate() {
                self.task_notification(out, r, &format!("{base}-tag-{i}"), tag.inner);
            }
            return;
        }
        if starts_with_tag(&prompt, "teammate-message") {
            // Delivered (again) as a user record; that one is decoded.
            return;
        }
        if !prompt.trim().is_empty() {
            self.notification(
                out,
                r,
                base,
                format!("Queued prompt: {prompt}"),
                "info",
                "queued_command",
            );
        }
    }

    // ---- system ----

    fn on_system(&mut self, rec: SystemRecord, out: &mut Vec<AgentEvent>) {
        let r = self.rec(&rec.env);
        let base = r.base.clone();
        let subtype = rec.subtype.clone().unwrap_or_default();
        match subtype.as_str() {
            "turn_duration" => {
                self.push(
                    out,
                    &r,
                    &base,
                    SemanticSuffix::TurnEnd,
                    EventPayload::TurnEnd(TurnEndPayload {
                        outcome: TurnOutcome::Completed,
                        duration_ms: rec.duration_ms,
                        turn_id: None,
                        pending_background_agents: rec.pending_background_agent_count,
                    }),
                );
            }
            "compact_boundary" => {
                let meta = rec.compact_metadata.unwrap_or_default();
                let trigger = match meta.trigger.as_deref() {
                    Some("auto") => CompactionTrigger::Auto,
                    Some("manual") => CompactionTrigger::Manual,
                    _ => CompactionTrigger::Unknown,
                };
                self.push(
                    out,
                    &r,
                    &base,
                    SemanticSuffix::Compaction,
                    EventPayload::Compaction(CompactionPayload {
                        trigger,
                        pre_tokens: meta.pre_tokens,
                        post_tokens: meta.post_tokens,
                        window_number: None,
                        duration_ms: meta.duration_ms,
                    }),
                );
            }
            "agents_killed" => {
                self.lifecycle(
                    out,
                    &r,
                    &base,
                    AgentHandle::Unknown("background_agents".to_string()),
                    LifecycleTransition::AllBackgroundKilled,
                    None,
                    None,
                );
            }
            "local_command" => self.local_command(out, &r, &base, &rec),
            "stop_hook_summary" => {
                let commands: Vec<String> = rec
                    .hook_infos
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|h| h.command)
                    .collect();
                let count = rec.hook_count.unwrap_or(commands.len() as u32);
                let text = if commands.is_empty() {
                    format!("Stop hooks executed (count: {count})")
                } else {
                    format!(
                        "Stop hooks executed (count: {count}): {}",
                        commands.join(", ")
                    )
                };
                self.notification(out, &r, &base, text, "info", &subtype);
            }
            "away_summary" => {
                if let Some(content) = rec.content {
                    self.notification(
                        out,
                        &r,
                        &base,
                        format!("Away summary: {content}"),
                        "info",
                        &subtype,
                    );
                }
            }
            "informational" => {
                if let Some(content) = rec.content {
                    let level = rec.level.unwrap_or_else(|| "info".to_string());
                    self.notification(out, &r, &base, content, &level, &subtype);
                }
            }
            "api_error" => {
                let text = match (rec.retry_attempt, rec.max_retries) {
                    (Some(attempt), Some(max)) => format!("API error (retry {attempt}/{max})"),
                    _ => rec.content.unwrap_or_else(|| "API error".to_string()),
                };
                self.notification(out, &r, &base, text, "warn", &subtype);
            }
            "scheduled_task_fire" => {
                let text = rec
                    .content
                    .or(rec.prompt)
                    .unwrap_or_else(|| "Scheduled task fired".to_string());
                self.notification(out, &r, &base, text, "info", &subtype);
            }
            other => self.diagnostics.record_unknown(&format!("system/{other}")),
        }
    }

    fn local_command(
        &mut self,
        out: &mut Vec<AgentEvent>,
        r: &Rec,
        base: &str,
        rec: &SystemRecord,
    ) {
        if let Some(command) = rec.command_run.as_ref().and_then(|c| c.command.clone()) {
            let name = if command.starts_with('/') {
                command
            } else {
                format!("/{command}")
            };
            let args = rec
                .command_run
                .as_ref()
                .and_then(|c| c.args.clone())
                .filter(|a| !a.is_empty());
            self.push(
                out,
                r,
                base,
                SemanticSuffix::SlashCommand,
                EventPayload::SlashCommand(SlashCommandPayload { name, args }),
            );
            return;
        }
        let Some(content) = rec.content.as_deref() else {
            return;
        };
        if let Some((name, args)) = slash_command(content) {
            self.push(
                out,
                r,
                base,
                SemanticSuffix::SlashCommand,
                EventPayload::SlashCommand(SlashCommandPayload { name, args }),
            );
        } else if starts_with_tag(content, "local-command-stdout") {
            self.local_command_output(out, r, base, content);
        } else if content.starts_with('/') {
            let (name, args) = match content.split_once(' ') {
                Some((n, a)) => (n.to_string(), Some(a.to_string())),
                None => (content.to_string(), None),
            };
            self.push(
                out,
                r,
                base,
                SemanticSuffix::SlashCommand,
                EventPayload::SlashCommand(SlashCommandPayload { name, args }),
            );
        }
    }

    // ---- dispatch ----

    fn map_record(&mut self, record: ClaudeRecord, text: &str, out: &mut Vec<AgentEvent>) {
        if let Some(env) = record.envelope() {
            self.runtime_alias(out, env);
        }
        match record {
            ClaudeRecord::User(r) => self.on_user(r, out),
            ClaudeRecord::Assistant(r) => self.on_assistant(r, out),
            ClaudeRecord::Attachment(r) => self.on_attachment(r, text, out),
            ClaudeRecord::System(r) => self.on_system(r, out),
            ClaudeRecord::AgentName(s) => {
                let r = self.bare_rec(None);
                self.attribute(out, &r, AgentAttributeKey::AgentName, s.agent_name);
            }
            ClaudeRecord::AiTitle(s) => {
                let r = self.bare_rec(None);
                self.attribute(out, &r, AgentAttributeKey::Title, s.ai_title);
            }
            ClaudeRecord::AgentSetting(s) => {
                let r = self.bare_rec(None);
                self.attribute(out, &r, AgentAttributeKey::AgentType, s.agent_setting);
            }
            ClaudeRecord::PermissionMode(s) => {
                let r = self.bare_rec(None);
                self.attribute(
                    out,
                    &r,
                    AgentAttributeKey::PermissionMode,
                    s.permission_mode,
                );
            }
            ClaudeRecord::ContinuedIn(s) => {
                let r = self.bare_rec(s.timestamp.as_deref());
                self.attribute(
                    out,
                    &r,
                    AgentAttributeKey::ContinuedIn,
                    s.continued_in_session_id,
                );
            }
            ClaudeRecord::CostState(s) => {
                let r = self.bare_rec(None);
                for model in s.model_usage.keys().filter(|k| k.ends_with("[1m]")) {
                    self.extended_marker(
                        out,
                        &r,
                        "self",
                        model,
                        "cost-state modelUsage".to_string(),
                    );
                }
            }
            ClaudeRecord::QueueOperation(q) => {
                let r = self.bare_rec(q.timestamp.as_deref());
                let base = r.base.clone();
                self.push(
                    out,
                    &r,
                    &base,
                    SemanticSuffix::QueueOperation,
                    EventPayload::QueueOperation(QueueOperationPayload {
                        operation: q.operation.unwrap_or_default(),
                        content: q.content,
                        task_id: q.task_id,
                        reason: q.reason,
                    }),
                );
            }
            ClaudeRecord::PrLink(p) => {
                let r = self.bare_rec(p.timestamp.as_deref());
                let base = r.base.clone();
                let text = match (p.pr_number, p.pr_url) {
                    (Some(n), Some(url)) => format!("PR #{n} linked: {url}"),
                    (Some(n), None) => format!("PR #{n} linked"),
                    (None, Some(url)) => format!("PR linked: {url}"),
                    (None, None) => "PR linked".to_string(),
                };
                self.notification(out, &r, &base, text, "info", "pr_link");
            }
            ClaudeRecord::Ignored(_) => self.diagnostics.record_ignored(&kind_label_of(text)),
            ClaudeRecord::Unknown => {
                let kind = kind_label_of(text);
                if kind.starts_with("artifact-") {
                    self.diagnostics.record_ignored(&kind);
                } else {
                    self.diagnostics.record_unknown(&kind);
                }
            }
        }
    }
}

/// Claude usage -> TokenUsagePayload.
///
/// Input: `uncached = input_tokens`, `cache_read = cache_read_input_tokens`,
/// `cache_write = cache_creation_input_tokens`. Output: `output_tokens` already
/// includes `thinking_tokens`, which is carved out as `reasoning`.
/// Stream-start write mode (`stop_reason == null` and no `iterations`) only has a
/// message_start snapshot of the output: `PartialOutput`.
fn usage_payload(usage: &Usage, stop_reason_missing: bool) -> TokenUsagePayload {
    let input = TokenInput::new(
        usage.input_tokens,
        usage.cache_read_input_tokens.unwrap_or(0),
        usage.cache_creation_input_tokens.unwrap_or(0),
    );
    let reasoning = usage
        .output_tokens_details
        .as_ref()
        .and_then(|d| d.thinking_tokens)
        .unwrap_or(0);
    let output = TokenOutput::new(usage.output_tokens.saturating_sub(reasoning), reasoning, 0);
    let has_iterations = usage.iterations.as_ref().is_some_and(|v| !v.is_null());
    let completeness = if stop_reason_missing && !has_iterations {
        UsageCompleteness::PartialOutput
    } else {
        UsageCompleteness::Final
    };
    TokenUsagePayload::new(input, output).with_completeness(completeness)
}

impl LogDecoder for ClaudeDecoder {
    fn decode_line(&mut self, line: RawLine<'_>) -> Vec<AgentEvent> {
        let Some(record) = decode_typed::<ClaudeRecord>(&line, &mut self.diagnostics) else {
            return Vec::new();
        };
        self.line = line.line;
        self.builder.begin_line(line.line, line.byte_offset);
        let mut out = Vec::new();
        self.map_record(record, line.text, &mut out);
        out
    }

    fn diagnostics(&self) -> &ParseDiagnostics {
        &self.diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_name_to_model_id() {
        assert_eq!(
            model_id_from_display_name("Opus 5.5 (1M context)").as_deref(),
            Some("claude-opus-5-5")
        );
        assert_eq!(
            model_id_from_display_name("Fable 5.1").as_deref(),
            Some("claude-fable-5-1")
        );
        assert_eq!(
            model_id_from_display_name("Opus 5").as_deref(),
            Some("claude-opus-5")
        );
        assert_eq!(model_id_from_display_name(" (x)"), None);
    }

    #[test]
    fn base_model_ids() {
        assert_eq!(base_model_id("claude-opus-5-5[1m]"), "claude-opus-5-5");
        assert_eq!(base_model_id("claude-opus-5-5"), "claude-opus-5-5");
        assert_eq!(base_model_id("odd[x"), "odd[x");
    }

    #[test]
    fn attachment_labels() {
        assert_eq!(
            attachment_label(
                r#"{"type":"attachment","attachment":{"type":"skill_listing","x":1}}"#
            ),
            "attachment/skill_listing"
        );
        assert_eq!(attachment_label(r#"{"type":"attachment"}"#), "attachment");
    }

    #[test]
    fn usage_completeness() {
        let mut u = Usage {
            input_tokens: 1,
            output_tokens: 10,
            ..Default::default()
        };
        assert_eq!(
            usage_payload(&u, true).completeness,
            UsageCompleteness::PartialOutput
        );
        assert_eq!(
            usage_payload(&u, false).completeness,
            UsageCompleteness::Final
        );
        u.iterations = Some(serde_json::json!([]));
        assert_eq!(
            usage_payload(&u, true).completeness,
            UsageCompleteness::Final
        );
    }

    #[test]
    fn recent_map_forgets_oldest() {
        let mut m = RecentMap::new();
        for i in 0..=RECENT_MESSAGES {
            m.insert(i.to_string(), i);
        }
        assert!(m.get("0").is_none());
        assert_eq!(m.get(&RECENT_MESSAGES.to_string()), Some(&RECENT_MESSAGES));
    }
}
