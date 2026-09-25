//! Line-oriented Codex rollout decoder (`LogDecoder`) for Codex ≥ 0.153
//! (paginated rollouts, multi_agent v2).
//!
//! Rules (DESIGN §2.3):
//! - **Fork prefix:** a forked child embeds a copy of its parent's history; records with
//!   `ordinal < subagent_history_start_ordinal` are skipped (`ignored_kinds["fork_prefix"]`).
//! - **Twins:** `response_item` is primary for messages, reasoning and tool calls; the
//!   `item_completed` twins (Reasoning / AgentMessage / UserMessage / ContextCompaction) are
//!   ignored.
//! - **Usage** comes from `token_usage_record` (one per response, deduped by `response_id`);
//!   `event_msg.token_count` only contributes the context window.
//! - **exec sub-actions** (`item_completed` CommandExecution / FileChange / ImageView /
//!   Extension / McpToolCall) become `ToolSubAction`s correlated to the enclosing `exec` call.
//! - **Collaboration tools** produce an Agent `ToolCall` plus an outgoing `AgentMessage`;
//!   `SubAgentActivity` in the parent yields `AgentSpawn` / `AgentLifecycle`; received
//!   `agent_message`s yield incoming `AgentMessage`s.

use agtrace_types::*;
use chrono::{DateTime, Utc};
use regex::Regex;
use serde::de::DeserializeOwned;
use serde_json::Value;
use serde_json::value::RawValue;
use std::collections::HashMap;
use std::sync::LazyLock;
use uuid::Uuid;

use super::collab;
use super::exec::{self, ExecTracker};
use super::records::{self as rec, *};
use crate::builder::{EventBuilder, SemanticSuffix};
use crate::lenient::RawLine;
use crate::provider::{DecodeOptions, FileHeader, LogDecoder};

/// "Exit code: N" in legacy tool outputs.
static EXIT_CODE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)Exit Code:\s*(\d+)").unwrap());

/// Maximum size of sub-action output previews.
const MAX_PREVIEW_BYTES: usize = 2048;

const REASONING_ENCRYPTED: &str = "[reasoning encrypted]";

/// Spawn arguments remembered until the matching `SubAgentActivity(started)`.
#[derive(Debug, Clone)]
struct PendingSpawn {
    fork: bool,
    model: Option<String>,
}

pub struct CodexDecoder {
    builder: EventBuilder,
    agent: AgentId,
    /// Native thread id (prefix of line-based event ids).
    thread_key: String,
    /// This agent's path (`/root` for roots).
    self_path: String,
    /// Handle of the parent (children only), target of FINAL_ANSWERs.
    parent: Option<AgentHandle>,
    diagnostics: ParseDiagnostics,

    /// Set by the first `session_meta` of a forked child.
    history_start: Option<u64>,
    seen_session_meta: bool,
    last_timestamp: Option<DateTime<Utc>>,
    model: Option<String>,
    context_window: Option<u64>,
    turn_id: Option<String>,
    /// response_id -> usage already emitted (upsert when it changes).
    usage_seen: HashMap<String, Usage>,
    /// `inter_agent_communication_metadata.trigger_turn` for the next agent_message.
    pending_trigger_turn: Option<bool>,
    /// spawn_agent call_id -> spawn args.
    pending_spawns: HashMap<String, PendingSpawn>,
    exec: ExecTracker,
}

impl CodexDecoder {
    pub fn new(header: &FileHeader, opts: DecodeOptions) -> Self {
        let thread_key = header.agent.native_session_id.clone();
        let session_uuid = Uuid::new_v5(&Uuid::NAMESPACE_OID, thread_key.as_bytes());
        let self_path = header
            .agent
            .path
            .clone()
            .unwrap_or_else(|| collab::ROOT_PATH.to_string());
        let parent = match header.agent.kind {
            AgentKind::Main => None,
            _ => collab::parent_path(&self_path)
                .map(AgentHandle::Path)
                .or_else(|| header.agent.parent.clone().map(AgentHandle::Id)),
        };
        Self {
            builder: EventBuilder::new(session_uuid),
            agent: header.agent.id.clone(),
            thread_key,
            self_path,
            parent,
            diagnostics: ParseDiagnostics::default(),
            history_start: None,
            seen_session_meta: false,
            last_timestamp: opts.fallback_timestamp,
            model: None,
            context_window: None,
            turn_id: None,
            usage_seen: HashMap::new(),
            pending_trigger_turn: None,
            pending_spawns: HashMap::new(),
            exec: ExecTracker::default(),
        }
    }

    /// Record timestamp; missing / unparsable timestamps inherit the last seen one.
    fn timestamp(&mut self, ts: Option<&str>) -> DateTime<Utc> {
        if let Some(dt) = ts.and_then(|t| DateTime::parse_from_rfc3339(t).ok()) {
            let dt = dt.with_timezone(&Utc);
            self.last_timestamp = Some(dt);
            return dt;
        }
        self.last_timestamp.unwrap_or(DateTime::<Utc>::UNIX_EPOCH)
    }

    /// Typed payload decode; failure is counted as a schema mismatch of `kind`.
    fn typed<T: DeserializeOwned>(
        &mut self,
        raw: Option<&RawValue>,
        kind: &str,
        line: &RawLine<'_>,
    ) -> Option<T> {
        let text = raw.map(RawValue::get).unwrap_or("null");
        match serde_json::from_str::<T>(text) {
            Ok(v) => Some(v),
            Err(e) => {
                self.diagnostics.record_schema_mismatch(
                    kind,
                    line.line,
                    line.byte_offset,
                    e.to_string(),
                );
                None
            }
        }
    }

    fn sub_type(raw: Option<&RawValue>) -> Option<String> {
        raw.and_then(|r| serde_json::from_str::<TypeTag>(r.get()).ok())
            .and_then(|t| t.kind.map(|k| k.into_owned()))
    }
}

/// Per-line emission context.
struct Emit<'a> {
    events: &'a mut Vec<AgentEvent>,
    base_id: String,
    ts: DateTime<Utc>,
}

impl CodexDecoder {
    fn push(&mut self, cx: &mut Emit<'_>, suffix: SemanticSuffix, payload: EventPayload) -> Uuid {
        let base_id = cx.base_id.clone();
        self.push_with_base(cx, &base_id, suffix, payload)
    }

    fn push_with_base(
        &mut self,
        cx: &mut Emit<'_>,
        base_id: &str,
        suffix: SemanticSuffix,
        payload: EventPayload,
    ) -> Uuid {
        let agent = self.agent.clone();
        self.builder
            .build_and_push(cx.events, base_id, suffix, cx.ts, payload, &agent)
    }

    fn self_handle(&self) -> AgentHandle {
        AgentHandle::Id(self.agent.clone())
    }

    fn set_model(&mut self, cx: &mut Emit<'_>, model: Option<String>, source: ModelChangeSource) {
        let Some(model) = model.filter(|m| !m.is_empty()) else {
            return;
        };
        if self.model.as_deref() == Some(model.as_str()) {
            return;
        }
        let from = self.model.replace(model.clone());
        self.push(
            cx,
            SemanticSuffix::ModelChange,
            EventPayload::ModelChange(ModelChangePayload {
                from,
                to: model,
                source,
            }),
        );
    }

    fn set_context_window(&mut self, cx: &mut Emit<'_>, window: Option<u64>) {
        let Some(window) = window else { return };
        if self.context_window == Some(window) {
            return;
        }
        self.context_window = Some(window);
        let model = self.model.clone();
        self.push(
            cx,
            SemanticSuffix::ContextWindowHint,
            EventPayload::ContextWindowHint(ContextWindowHintPayload::Explicit {
                tokens: window,
                model,
            }),
        );
    }

    fn lifecycle(&mut self, cx: &mut Emit<'_>, target: AgentHandle, t: LifecycleTransition) {
        self.push(
            cx,
            SemanticSuffix::AgentLifecycle,
            EventPayload::AgentLifecycle(AgentLifecyclePayload {
                target,
                transition: t,
                reason: None,
                usage: None,
            }),
        );
    }

    fn turn_end(
        &mut self,
        cx: &mut Emit<'_>,
        outcome: TurnOutcome,
        turn_id: Option<String>,
        duration_ms: Option<u64>,
    ) {
        self.push(
            cx,
            SemanticSuffix::TurnEnd,
            EventPayload::TurnEnd(TurnEndPayload {
                outcome,
                duration_ms,
                turn_id: turn_id.or_else(|| self.turn_id.clone()),
                pending_background_agents: None,
            }),
        );
    }

    // ------------------------------------------------------------------
    // event_msg
    // ------------------------------------------------------------------

    fn event_msg(&mut self, cx: &mut Emit<'_>, payload: Option<&RawValue>, line: &RawLine<'_>) {
        let sub = Self::sub_type(payload).unwrap_or_default();
        let kind = format!("event_msg/{sub}");
        match sub.as_str() {
            "task_started" => {
                let Some(p) = self.typed::<TaskStarted>(payload, &kind, line) else {
                    return;
                };
                if p.turn_id.is_some() {
                    self.turn_id = p.turn_id;
                }
                self.set_context_window(cx, p.model_context_window);
                let me = self.self_handle();
                self.lifecycle(cx, me, LifecycleTransition::Running);
            }
            "task_complete" => {
                let Some(p) = self.typed::<TaskComplete>(payload, &kind, line) else {
                    return;
                };
                let outcome = match p.error {
                    Some(err) => TurnOutcome::Failed { error: err.message },
                    None => TurnOutcome::Completed,
                };
                let failed = matches!(outcome, TurnOutcome::Failed { .. });
                // A child's final message is delivered to its parent as FINAL_ANSWER,
                // before the turn ends (nothing of the finished turn follows its end).
                if !failed
                    && let (Some(parent), Some(text)) = (self.parent.clone(), p.last_agent_message)
                {
                    let (body, encrypted) = collab::message_body(Some(&text));
                    self.push(
                        cx,
                        SemanticSuffix::AgentMessage,
                        EventPayload::AgentMessage(AgentMessagePayload {
                            direction: MessageDirection::Outgoing,
                            from: AgentHandle::Path(self.self_path.clone()),
                            to: vec![parent],
                            kind: AgentMessageKind::FinalAnswer,
                            body,
                            encrypted,
                            summary: None,
                            triggers_turn: None,
                            provider_message_id: None,
                        }),
                    );
                }
                self.turn_end(cx, outcome, p.turn_id, p.duration_ms);
            }
            "turn_aborted" => {
                let Some(p) = self.typed::<TurnAborted>(payload, &kind, line) else {
                    return;
                };
                self.turn_end(cx, TurnOutcome::Interrupted, p.turn_id, p.duration_ms);
            }
            "thread_settings_applied" => {
                let Some(p) = self.typed::<ThreadSettingsApplied>(payload, &kind, line) else {
                    return;
                };
                let model = p.thread_settings.and_then(|s| s.model);
                self.set_model(cx, model, ModelChangeSource::ThreadSettings);
            }
            "token_count" => {
                let Some(p) = self.typed::<rec::TokenCount>(payload, &kind, line) else {
                    return;
                };
                let window = p.info.and_then(|i| i.model_context_window);
                self.set_context_window(cx, window);
            }
            "item_completed" => self.item_completed(cx, payload, line),
            "thread_goal_updated" => self.diagnostics.record_ignored(&kind),
            _ => self.diagnostics.record_unknown(&kind),
        }
    }

    fn item_completed(
        &mut self,
        cx: &mut Emit<'_>,
        payload: Option<&RawValue>,
        line: &RawLine<'_>,
    ) {
        let Some(ic) = self.typed::<ItemCompleted>(payload, "event_msg/item_completed", line)
        else {
            return;
        };
        let item_type = Self::sub_type(Some(&ic.item)).unwrap_or_default();
        let kind = format!("event_msg/item_completed/{item_type}");
        let turn_id = ic.turn_id.clone().or_else(|| self.turn_id.clone());
        let parent = self.exec.attach(turn_id.as_deref());
        match item_type.as_str() {
            // Twins of response_item records / the `compacted` record.
            "Reasoning"
            | "AgentMessage"
            | "UserMessage"
            | "ContextCompaction"
            | "Plan"
            | "CollabAgentToolCall" => self.diagnostics.record_ignored(&kind),
            "CommandExecution" => {
                let Some(it) = self.typed::<CommandExecutionItem>(Some(&ic.item), &kind, line)
                else {
                    return;
                };
                let command = it.command.as_ref().and_then(shell_command_text);
                let output = it.aggregated_output.or(it.stdout);
                let sub = ToolSubActionPayload {
                    parent_tool_call_id: parent,
                    call: ToolCallPayload::Execute {
                        name: "exec_command".to_string(),
                        arguments: ExecuteArgs {
                            command,
                            description: None,
                            timeout: None,
                            extra: serde_json::json!({}),
                        },
                        provider_call_id: None,
                    },
                    status: sub_status(it.status.as_deref()),
                    exit_code: it.exit_code,
                    output_preview: preview(output.as_deref()),
                    duration_ms: it.duration.map(|d| d.as_millis()),
                };
                self.push(
                    cx,
                    SemanticSuffix::ToolSubAction,
                    EventPayload::ToolSubAction(sub),
                );
            }
            "FileChange" => {
                let Some(it) = self.typed::<FileChangeItem>(Some(&ic.item), &kind, line) else {
                    return;
                };
                let status = sub_status(it.status.as_deref());
                let output = if status == SubActionStatus::Failed {
                    it.stderr.filter(|s| !s.is_empty()).or(it.stdout)
                } else {
                    it.stdout
                };
                for (i, (path, change)) in it.changes.into_iter().enumerate() {
                    let call = file_change_call(path, change);
                    let sub = ToolSubActionPayload {
                        parent_tool_call_id: parent,
                        call,
                        status,
                        exit_code: None,
                        output_preview: preview(output.as_deref()),
                        duration_ms: None,
                    };
                    let base = if i == 0 {
                        cx.base_id.clone()
                    } else {
                        format!("{}:{i}", cx.base_id)
                    };
                    self.push_with_base(
                        cx,
                        &base,
                        SemanticSuffix::ToolSubAction,
                        EventPayload::ToolSubAction(sub),
                    );
                }
            }
            "ImageView" => {
                let Some(it) = self.typed::<ImageViewItem>(Some(&ic.item), &kind, line) else {
                    return;
                };
                let sub = ToolSubActionPayload {
                    parent_tool_call_id: parent,
                    call: ToolCallPayload::FileRead {
                        name: "view_image".to_string(),
                        arguments: FileReadArgs {
                            file_path: it.path,
                            path: None,
                            pattern: None,
                            extra: serde_json::json!({}),
                        },
                        provider_call_id: None,
                    },
                    status: SubActionStatus::Completed,
                    exit_code: None,
                    output_preview: None,
                    duration_ms: None,
                };
                self.push(
                    cx,
                    SemanticSuffix::ToolSubAction,
                    EventPayload::ToolSubAction(sub),
                );
            }
            "Extension" => {
                let Some(it) = self.typed::<ExtensionItem>(Some(&ic.item), &kind, line) else {
                    return;
                };
                let ext = it.kind.clone().unwrap_or_else(|| "extension".to_string());
                let call = if ext == "web.search" {
                    let query = it.query.clone().or_else(|| {
                        it.action
                            .as_ref()
                            .and_then(|a| a.get("query"))
                            .and_then(Value::as_str)
                            .map(str::to_string)
                    });
                    ToolCallPayload::Search {
                        name: ext,
                        arguments: SearchArgs {
                            pattern: None,
                            query,
                            input: None,
                            path: None,
                            extra: serde_json::json!({}),
                        },
                        provider_call_id: None,
                    }
                } else {
                    let mut args = serde_json::Map::new();
                    if let Some(ms) = it.duration_ms {
                        args.insert("durationMs".into(), ms.into());
                    }
                    ToolCallPayload::Generic {
                        name: ext,
                        arguments: Value::Object(args),
                        provider_call_id: None,
                    }
                };
                let sub = ToolSubActionPayload {
                    parent_tool_call_id: parent,
                    call,
                    status: sub_status(it.status.as_deref()),
                    exit_code: None,
                    output_preview: None,
                    duration_ms: it.duration_ms,
                };
                self.push(
                    cx,
                    SemanticSuffix::ToolSubAction,
                    EventPayload::ToolSubAction(sub),
                );
            }
            "McpToolCall" => {
                let Some(it) = self.typed::<McpToolCallItem>(Some(&ic.item), &kind, line) else {
                    return;
                };
                let server = it.server.unwrap_or_default();
                let tool = it.tool.unwrap_or_default();
                let is_error = it.result.as_ref().and_then(|r| r.is_error) == Some(true);
                let output = it.result.map(|r| {
                    r.content
                        .iter()
                        .filter_map(|c| c.text.as_deref())
                        .collect::<Vec<_>>()
                        .join("\n")
                });
                let mut status = sub_status(it.status.as_deref());
                if is_error {
                    status = SubActionStatus::Failed;
                }
                let sub = ToolSubActionPayload {
                    parent_tool_call_id: parent,
                    call: ToolCallPayload::Mcp {
                        name: format!("mcp__{server}__{tool}"),
                        arguments: McpArgs {
                            server: Some(server).filter(|s| !s.is_empty()),
                            tool: Some(tool).filter(|s| !s.is_empty()),
                            inner: it.arguments.unwrap_or_else(|| serde_json::json!({})),
                        },
                        provider_call_id: None,
                    },
                    status,
                    exit_code: None,
                    output_preview: preview(output.as_deref()),
                    duration_ms: it.duration.map(|d| d.as_millis()),
                };
                self.push(
                    cx,
                    SemanticSuffix::ToolSubAction,
                    EventPayload::ToolSubAction(sub),
                );
            }
            "SubAgentActivity" => {
                let Some(it) = self.typed::<SubAgentActivityItem>(Some(&ic.item), &kind, line)
                else {
                    return;
                };
                self.sub_agent_activity(cx, it);
            }
            _ => self.diagnostics.record_unknown(&kind),
        }
    }

    fn sub_agent_activity(&mut self, cx: &mut Emit<'_>, it: SubAgentActivityItem) {
        let child = match (&it.agent_thread_id, &it.agent_path) {
            (Some(tid), _) if !tid.is_empty() => AgentHandle::Id(AgentId::codex_thread(tid)),
            (_, Some(path)) => AgentHandle::Path(path.clone()),
            _ => AgentHandle::Unknown(it.id.clone().unwrap_or_default()),
        };
        let transition = match it.kind.as_deref() {
            Some("started") => {
                let spawn = it
                    .id
                    .as_deref()
                    .and_then(|id| self.pending_spawns.get(id))
                    .cloned();
                let kind = if spawn.as_ref().is_some_and(|s| s.fork) {
                    AgentKind::Fork
                } else {
                    AgentKind::CodexThread
                };
                let tool_call_id = it
                    .id
                    .as_deref()
                    .and_then(|id| self.builder.get_tool_call_uuid(id));
                self.push(
                    cx,
                    SemanticSuffix::AgentSpawn,
                    EventPayload::AgentSpawn(AgentSpawnPayload {
                        child,
                        kind,
                        name: it.agent_path.as_deref().and_then(collab::path_leaf),
                        agent_type: None,
                        requested_model: spawn.and_then(|s| s.model),
                        resolved_model: None,
                        description: None,
                        spawn_call_id: it.id,
                        tool_call_id,
                    }),
                );
                return;
            }
            Some("interacted") => LifecycleTransition::Running,
            Some("interrupted") => LifecycleTransition::Interrupted,
            Some("completed") => LifecycleTransition::Completed,
            other => {
                self.diagnostics.record_unknown(&format!(
                    "event_msg/item_completed/SubAgentActivity/{}",
                    other.unwrap_or("")
                ));
                return;
            }
        };
        self.lifecycle(cx, child, transition);
    }

    // ------------------------------------------------------------------
    // response_item
    // ------------------------------------------------------------------

    fn response_item(&mut self, cx: &mut Emit<'_>, payload: Option<&RawValue>, line: &RawLine<'_>) {
        let sub = Self::sub_type(payload).unwrap_or_default();
        let kind = format!("response_item/{sub}");
        match sub.as_str() {
            "message" => {
                let Some(m) = self.typed::<MessageItem>(payload, &kind, line) else {
                    return;
                };
                self.message(cx, m, &kind);
            }
            "reasoning" => {
                let Some(r) = self.typed::<ReasoningItem>(payload, &kind, line) else {
                    return;
                };
                let mut text = r
                    .summary
                    .iter()
                    .filter_map(|s| s.text.as_deref())
                    .collect::<Vec<_>>()
                    .join("\n");
                if text.is_empty()
                    && let Some(Value::String(c)) = &r.content
                {
                    text = c.clone();
                }
                if text.is_empty() {
                    text = REASONING_ENCRYPTED.to_string();
                }
                self.push(
                    cx,
                    SemanticSuffix::Reasoning,
                    EventPayload::Reasoning(ReasoningPayload { text }),
                );
            }
            "function_call" => {
                let Some(fc) = self.typed::<FunctionCallItem>(payload, &kind, line) else {
                    return;
                };
                self.function_call(cx, fc);
            }
            "function_call_output" => {
                let Some(o) = self.typed::<ToolOutputItem>(payload, &kind, line) else {
                    return;
                };
                let is_wait = self.exec.on_wait_output(&o.call_id, &o.output);
                let is_error = if self.pending_spawns.contains_key(&o.call_id) {
                    collab::is_failed_spawn_output(&o.output)
                } else if is_wait {
                    exec::is_failed_exec_output(&o.output)
                } else {
                    exit_code(&o.output).is_some_and(|c| c != 0)
                };
                self.tool_result(cx, &o.call_id, o.output, is_error);
            }
            "custom_tool_call" => {
                let Some(c) = self.typed::<CustomToolCallItem>(payload, &kind, line) else {
                    return;
                };
                let turn_id = c
                    .internal_chat_message_metadata_passthrough
                    .as_ref()
                    .and_then(|p| p.turn_id.clone())
                    .or_else(|| self.turn_id.clone());
                let args = if c.name == "exec" {
                    Value::String(c.input.clone())
                } else {
                    serde_json::from_str::<Value>(&c.input)
                        .unwrap_or_else(|_| serde_json::json!({ "raw": c.input }))
                };
                let call = super::mapper::normalize_codex_tool_call(
                    c.name.clone(),
                    args,
                    Some(c.call_id.clone()),
                );
                let id = self.push(cx, SemanticSuffix::ToolCall, EventPayload::ToolCall(call));
                self.builder.register_tool_call(c.call_id.clone(), id);
                if c.name == "exec" {
                    self.exec.on_call(turn_id, &c.call_id, id);
                }
            }
            "custom_tool_call_output" => {
                let Some(o) = self.typed::<ToolOutputItem>(payload, &kind, line) else {
                    return;
                };
                self.exec.on_output(&o.call_id, &o.output);
                let is_error = exec::is_failed_exec_output(&o.output);
                self.tool_result(cx, &o.call_id, o.output, is_error);
            }
            "agent_message" => {
                let Some(m) = self.typed::<AgentMessageItem>(payload, &kind, line) else {
                    return;
                };
                let parsed = collab::parse_received_message(&m.content);
                let from = m
                    .author
                    .map(AgentHandle::Path)
                    .unwrap_or_else(|| AgentHandle::Unknown(String::new()));
                let to = vec![AgentHandle::Path(
                    m.recipient.unwrap_or_else(|| self.self_path.clone()),
                )];
                let triggers_turn = self.pending_trigger_turn.take();
                self.push(
                    cx,
                    SemanticSuffix::AgentMessage,
                    EventPayload::AgentMessage(AgentMessagePayload {
                        direction: MessageDirection::Incoming,
                        from,
                        to,
                        kind: parsed.kind,
                        body: parsed.body,
                        encrypted: parsed.encrypted,
                        summary: None,
                        triggers_turn,
                        provider_message_id: m.id,
                    }),
                );
            }
            _ => self.diagnostics.record_unknown(&kind),
        }
    }

    fn message(&mut self, cx: &mut Emit<'_>, m: MessageItem, kind: &str) {
        match m.role.as_str() {
            "assistant" => {
                let text = join_text(m.content.iter());
                self.push(
                    cx,
                    SemanticSuffix::Message,
                    EventPayload::Message(MessagePayload {
                        text,
                        phase: m.phase,
                    }),
                );
            }
            "user" => {
                let kinds = m
                    .internal_chat_message_metadata_passthrough
                    .and_then(|p| p.content_item_kinds)
                    .unwrap_or_default();
                if !kinds.iter().any(|k| k == "user.text") {
                    // Injected context (environment, AGENTS.md, goals, ...).
                    self.diagnostics
                        .record_ignored(&format!("{kind}/user(injected)"));
                    return;
                }
                // content_item_kinds is parallel to content: keep only user.text items.
                let text = if kinds.len() == m.content.len() {
                    join_text(
                        m.content
                            .iter()
                            .zip(kinds.iter())
                            .filter(|(_, k)| *k == "user.text")
                            .map(|(c, _)| c),
                    )
                } else {
                    join_text(m.content.iter())
                };
                self.push(
                    cx,
                    SemanticSuffix::User,
                    EventPayload::User(UserPayload { text }),
                );
            }
            role => {
                // developer instructions, model_switch, turn_aborted markers, ...
                self.diagnostics.record_ignored(&format!("{kind}/{role}"));
            }
        }
    }

    fn function_call(&mut self, cx: &mut Emit<'_>, fc: FunctionCallItem) {
        let raw_args = fc.arguments.unwrap_or_default();
        let args = if raw_args.trim().is_empty() {
            serde_json::json!({})
        } else {
            serde_json::from_str::<Value>(&raw_args)
                .unwrap_or_else(|_| serde_json::json!({ "raw": raw_args }))
        };
        let is_collab = collab::agent_op(&fc.name).is_some()
            && fc
                .namespace
                .as_deref()
                .is_none_or(|ns| ns == "collaboration");

        if fc.name == "wait"
            && let Some(cell) = args.get("cell_id").and_then(Value::as_u64)
        {
            self.exec.on_wait_call(&fc.call_id, cell);
        }

        let call = match collab::agent_tool_args(&fc.name, &args).filter(|_| is_collab) {
            Some(agent_args) => ToolCallPayload::Agent {
                name: fc.name.clone(),
                arguments: agent_args,
                provider_call_id: Some(fc.call_id.clone()),
            },
            None => super::mapper::normalize_codex_tool_call(
                fc.name.clone(),
                args.clone(),
                Some(fc.call_id.clone()),
            ),
        };
        let call_uuid = self.push(cx, SemanticSuffix::ToolCall, EventPayload::ToolCall(call));
        self.builder
            .register_tool_call(fc.call_id.clone(), call_uuid);

        if !is_collab {
            return;
        }
        if fc.name == "spawn_agent" {
            self.pending_spawns.insert(
                fc.call_id.clone(),
                PendingSpawn {
                    fork: args.get("fork_turns").and_then(Value::as_str) == Some("all"),
                    model: args
                        .get("model")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                },
            );
        }
        let Some(msg_kind) = collab::outgoing_message_kind(&fc.name) else {
            return;
        };
        let target = if fc.name == "spawn_agent" {
            args.get("task_name").and_then(Value::as_str)
        } else {
            args.get("target").and_then(Value::as_str)
        };
        let to = target
            .map(|t| AgentHandle::Path(collab::resolve_target(&self.self_path, t)))
            .into_iter()
            .collect();
        let (body, encrypted) = collab::message_body(args.get("message").and_then(Value::as_str));
        self.push(
            cx,
            SemanticSuffix::AgentMessage,
            EventPayload::AgentMessage(AgentMessagePayload {
                direction: MessageDirection::Outgoing,
                from: AgentHandle::Path(self.self_path.clone()),
                to,
                kind: msg_kind,
                body,
                encrypted,
                summary: None,
                triggers_turn: None,
                provider_message_id: None,
            }),
        );
    }

    fn tool_result(&mut self, cx: &mut Emit<'_>, call_id: &str, output: String, is_error: bool) {
        let Some(tool_call_id) = self.builder.get_tool_call_uuid(call_id) else {
            return;
        };
        self.push(
            cx,
            SemanticSuffix::ToolResult,
            EventPayload::ToolResult(ToolResultPayload {
                output,
                tool_call_id,
                is_error,
            }),
        );
    }

    // ------------------------------------------------------------------
    // top-level records
    // ------------------------------------------------------------------

    fn token_usage(&mut self, cx: &mut Emit<'_>, rec: TokenUsageRecord) {
        let Some(usage) = rec.usage else { return };
        if let Some(key) = &rec.response_id {
            if self.usage_seen.get(key) == Some(&usage) {
                return;
            }
            self.usage_seen.insert(key.clone(), usage);
        }
        let payload = TokenUsagePayload::new(
            TokenInput::new(
                usage.input_tokens.saturating_sub(usage.cached_input_tokens),
                usage.cached_input_tokens,
                usage.cache_write_input_tokens,
            ),
            TokenOutput::new(
                usage
                    .output_tokens
                    .saturating_sub(usage.reasoning_output_tokens),
                usage.reasoning_output_tokens,
                0,
            ),
        )
        .with_model(self.model.clone())
        .with_dedupe_key(rec.response_id.clone());
        // Deterministic id per response: v5(session, "{response_id}:usage").
        let base = rec.response_id.unwrap_or_else(|| cx.base_id.clone());
        self.push_with_base(
            cx,
            &base,
            SemanticSuffix::TokenUsage,
            EventPayload::TokenUsage(payload),
        );
    }
}

impl LogDecoder for CodexDecoder {
    fn decode_line(&mut self, line: RawLine<'_>) -> Vec<AgentEvent> {
        self.diagnostics.lines += 1;
        let text = line.text.trim();
        if text.is_empty() {
            return Vec::new();
        }
        let env = match serde_json::from_str::<Envelope>(text) {
            Ok(env) => env,
            Err(err) => {
                match serde_json::from_str::<Value>(text) {
                    Err(json_err) => self.diagnostics.record_invalid_json(
                        line.line,
                        line.byte_offset,
                        json_err.to_string(),
                    ),
                    Ok(v) => self.diagnostics.record_schema_mismatch(
                        &crate::lenient::kind_label(&v),
                        line.line,
                        line.byte_offset,
                        err.to_string(),
                    ),
                }
                return Vec::new();
            }
        };
        let kind = env.kind.as_deref().unwrap_or("").to_string();
        let ordinal = env.ordinal.unwrap_or(line.line);

        // Copied parent history of a forked child (incl. the parent's session_meta).
        if self.seen_session_meta
            && let Some(start) = self.history_start
            && ordinal < start
        {
            self.diagnostics.decoded += 1;
            self.diagnostics.record_ignored("fork_prefix");
            return Vec::new();
        }

        let mismatches_before = self.diagnostics.schema_mismatch_total();
        self.builder.begin_line(line.line, line.byte_offset);
        let ts = self.timestamp(env.timestamp.as_deref());
        let mut events = Vec::new();
        let mut cx = Emit {
            events: &mut events,
            base_id: format!("{}:row_{}", self.thread_key, line.line),
            ts,
        };

        match kind.as_str() {
            "session_meta" => {
                if !self.seen_session_meta {
                    self.seen_session_meta = true;
                    if let Some(meta) = self.typed::<SessionMeta>(env.payload, &kind, &line) {
                        self.history_start = meta.subagent_history_start_ordinal;
                    }
                } else {
                    // A second session_meta outside a fork prefix carries no new identity.
                    self.diagnostics.record_ignored("session_meta/duplicate");
                }
            }
            "turn_context" => {
                if let Some(tc) = self.typed::<TurnContext>(env.payload, &kind, &line) {
                    if tc.turn_id.is_some() {
                        self.turn_id = tc.turn_id;
                    }
                    self.set_model(&mut cx, tc.model, ModelChangeSource::TurnContext);
                }
            }
            "event_msg" => self.event_msg(&mut cx, env.payload, &line),
            "response_item" => self.response_item(&mut cx, env.payload, &line),
            "token_usage_record" => {
                if let Some(rec) = self.typed::<TokenUsageRecord>(env.payload, &kind, &line) {
                    self.token_usage(&mut cx, rec);
                }
            }
            "inter_agent_communication_metadata" => {
                if let Some(m) =
                    self.typed::<InterAgentCommunicationMetadata>(env.payload, &kind, &line)
                {
                    self.pending_trigger_turn = m.trigger_turn;
                }
            }
            "compacted" => {
                if let Some(c) = self.typed::<Compacted>(env.payload, &kind, &line) {
                    let pre_tokens = c
                        .latest_token_usage_record
                        .and_then(|r| r.usage)
                        .map(|u| u.input_tokens);
                    self.push(
                        &mut cx,
                        SemanticSuffix::Compaction,
                        EventPayload::Compaction(CompactionPayload {
                            trigger: CompactionTrigger::Auto,
                            pre_tokens,
                            post_tokens: None,
                            window_number: c.window_number,
                            duration_ms: None,
                        }),
                    );
                }
            }
            "world_state" => self.diagnostics.record_ignored(&kind),
            "" => self.diagnostics.record_unknown("<untyped>"),
            other => self.diagnostics.record_unknown(other),
        }

        if self.diagnostics.schema_mismatch_total() == mismatches_before {
            self.diagnostics.decoded += 1;
        }
        events
    }

    fn diagnostics(&self) -> &ParseDiagnostics {
        &self.diagnostics
    }
}

// ----------------------------------------------------------------------
// helpers
// ----------------------------------------------------------------------

fn join_text<'a>(items: impl Iterator<Item = &'a ContentItem>) -> String {
    items
        .filter_map(|c| c.text.as_deref())
        .collect::<Vec<_>>()
        .join("\n")
}

fn exit_code(output: &str) -> Option<i32> {
    EXIT_CODE_REGEX
        .captures(output)
        .and_then(|cap| cap.get(1))
        .and_then(|m| m.as_str().parse().ok())
}

fn preview(output: Option<&str>) -> Option<String> {
    output
        .filter(|s| !s.is_empty())
        .map(|s| collab::truncate_bytes(s, MAX_PREVIEW_BYTES))
}

fn sub_status(status: Option<&str>) -> SubActionStatus {
    match status {
        Some("failed") | Some("error") => SubActionStatus::Failed,
        _ => SubActionStatus::Completed,
    }
}

/// `["/bin/zsh", "-lc", "<cmd>"]` → `<cmd>`; other arrays are joined; strings kept.
fn shell_command_text(command: &Value) -> Option<String> {
    match command {
        Value::String(s) => Some(s.clone()),
        Value::Array(parts) => {
            let parts: Vec<&str> = parts.iter().filter_map(Value::as_str).collect();
            match parts.as_slice() {
                [_, flag, cmd] if flag.starts_with('-') && flag.ends_with('c') => {
                    Some((*cmd).to_string())
                }
                [] => None,
                other => Some(other.join(" ")),
            }
        }
        _ => None,
    }
}

fn file_change_call(path: String, change: FileChangeEntry) -> ToolCallPayload {
    match change.kind.as_deref() {
        Some("add") => ToolCallPayload::FileWrite {
            name: "apply_patch".to_string(),
            arguments: FileWriteArgs {
                file_path: path,
                content: change.content.unwrap_or_default(),
            },
            provider_call_id: None,
        },
        Some("update") | None => ToolCallPayload::FileEdit {
            name: "apply_patch".to_string(),
            arguments: FileEditArgs {
                file_path: path,
                old_string: String::new(),
                new_string: change.unified_diff.unwrap_or_default(),
                replace_all: false,
            },
            provider_call_id: None,
        },
        Some(other) => ToolCallPayload::Generic {
            name: "apply_patch".to_string(),
            arguments: serde_json::json!({ "file_path": path, "operation": other }),
            provider_call_id: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_command_unwraps_login_shell() {
        let v = serde_json::json!(["/bin/zsh", "-lc", "cargo test"]);
        assert_eq!(shell_command_text(&v).as_deref(), Some("cargo test"));
        let v = serde_json::json!(["ls", "-la"]);
        assert_eq!(shell_command_text(&v).as_deref(), Some("ls -la"));
        assert_eq!(
            shell_command_text(&serde_json::json!("echo hi")).as_deref(),
            Some("echo hi")
        );
    }

    #[test]
    fn exit_code_regex() {
        assert_eq!(exit_code("Exit code: 0"), Some(0));
        assert_eq!(exit_code("x\nExit Code: 127\n"), Some(127));
        assert_eq!(exit_code("none"), None);
    }
}
