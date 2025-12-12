use agtrace_types::{AgentEventV1, EventType, ToolStatus};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Turn {
    User {
        timestamp: String,
        content: String,
    },

    Agent {
        timestamp: String,
        chain: Vec<ChainItem>,
        outcome: TurnOutcome,
        stats: TurnStats,
    },

    System {
        timestamp: String,
        message: String,
        kind: SystemMessageKind,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChainItem {
    Thought {
        content: String,
    },

    Action {
        tool_name: String,
        input_summary: String,
        result: ActionResult,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ActionResult {
    Success { summary: String },
    Failure { error: String },
    Denied { reason: String },
    Interrupted,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum TurnOutcome {
    Completed,
    Interrupted { reason: String },
    Stuck,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TurnStats {
    pub total_tokens: Option<u64>,
    pub event_count: usize,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemMessageKind {
    Error,
    Warning,
    Info,
}

impl Turn {
    pub fn timestamp(&self) -> &str {
        match self {
            Turn::User { timestamp, .. } => timestamp,
            Turn::Agent { timestamp, .. } => timestamp,
            Turn::System { timestamp, .. } => timestamp,
        }
    }

    pub fn duration_ms(&self) -> Option<u64> {
        match self {
            Turn::Agent { stats, .. } => stats.duration_ms,
            _ => None,
        }
    }
}

pub fn interpret_turns(events: &[AgentEventV1]) -> Vec<Turn> {
    let mut turns = Vec::new();
    let mut buffer = AgentTurnBuffer::new();

    for event in events {
        match event.event_type {
            EventType::UserMessage => {
                if !buffer.is_empty() {
                    if let Some(turn) = buffer.to_turn() {
                        turns.push(turn);
                    }
                    buffer = AgentTurnBuffer::new();
                }

                let content = event.text.clone().unwrap_or_default();
                turns.push(Turn::User {
                    timestamp: event.ts.clone(),
                    content,
                });
            }

            EventType::SystemMessage => {
                if !buffer.is_empty() {
                    if let Some(turn) = buffer.to_turn() {
                        turns.push(turn);
                    }
                    buffer = AgentTurnBuffer::new();
                }

                let message = event.text.clone().unwrap_or_default();
                let kind = infer_system_message_kind(&message);
                turns.push(Turn::System {
                    timestamp: event.ts.clone(),
                    message,
                    kind,
                });
            }

            EventType::AssistantMessage => {
                if !buffer.is_empty() {
                    if let Some(turn) = buffer.to_turn() {
                        turns.push(turn);
                    }
                    buffer = AgentTurnBuffer::new();
                }
            }

            EventType::Reasoning => {
                buffer.mark_start(&event.ts);
                if let Some(content) = &event.text {
                    buffer.chain.push(ChainItem::Thought {
                        content: content.clone(),
                    });
                }
                buffer.event_count += 1;
            }

            EventType::ToolCall => {
                buffer.mark_start(&event.ts);
                buffer.push_tool_call(event);
                buffer.event_count += 1;
            }

            EventType::ToolResult => {
                buffer.update_tool_result(event);
                buffer.mark_end(&event.ts);
                buffer.event_count += 1;
            }

            _ => {}
        }
    }

    if !buffer.is_empty() {
        if let Some(turn) = buffer.to_turn() {
            turns.push(turn);
        }
    }

    turns
}

struct AgentTurnBuffer {
    chain: Vec<ChainItem>,
    pending_actions: Vec<PendingAction>,
    start_ts: Option<String>,
    end_ts: Option<String>,
    event_count: usize,
    total_tokens: Option<u64>,
}

struct PendingAction {
    call_id: Option<String>,
    index_in_chain: usize,
}

impl AgentTurnBuffer {
    fn new() -> Self {
        Self {
            chain: Vec::new(),
            pending_actions: Vec::new(),
            start_ts: None,
            end_ts: None,
            event_count: 0,
            total_tokens: None,
        }
    }

    fn is_empty(&self) -> bool {
        self.event_count == 0
    }

    fn mark_start(&mut self, ts: &str) {
        if self.start_ts.is_none() {
            self.start_ts = Some(ts.to_string());
        }
    }

    fn mark_end(&mut self, ts: &str) {
        self.end_ts = Some(ts.to_string());
    }

    fn push_tool_call(&mut self, event: &AgentEventV1) {
        if let Some(tool_name) = &event.tool_name {
            let input_summary = extract_input_summary(event);

            let index_in_chain = self.chain.len();
            self.chain.push(ChainItem::Action {
                tool_name: tool_name.clone(),
                input_summary,
                result: ActionResult::Missing,
            });

            self.pending_actions.push(PendingAction {
                call_id: event.tool_call_id.clone(),
                index_in_chain,
            });
        }
    }

    fn update_tool_result(&mut self, event: &AgentEventV1) {
        if let Some(call_id) = &event.tool_call_id {
            for (i, pending) in self.pending_actions.iter().enumerate().rev() {
                if pending.call_id.as_ref() == Some(call_id) {
                    let result = build_action_result(event);
                    if let Some(ChainItem::Action {
                        result: ref mut r, ..
                    }) = self.chain.get_mut(pending.index_in_chain)
                    {
                        *r = result;
                    }
                    self.pending_actions.remove(i);
                    break;
                }
            }
        }
    }

    fn to_turn(&self) -> Option<Turn> {
        if self.is_empty() {
            return None;
        }

        let duration_ms = calculate_duration(&self.start_ts, &self.end_ts);
        let outcome = determine_outcome(&self.chain);

        Some(Turn::Agent {
            timestamp: self.start_ts.clone().unwrap_or_default(),
            chain: self.chain.clone(),
            outcome,
            stats: TurnStats {
                total_tokens: self.total_tokens,
                event_count: self.event_count,
                duration_ms,
            },
        })
    }
}

fn extract_input_summary(event: &AgentEventV1) -> String {
    if let Some(file_path) = &event.file_path {
        if let Some(filename) = std::path::Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
        {
            return filename.to_string();
        }
    }

    if let Some(text) = &event.text {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(text) {
            if let Some(cmd) = json.get("command").and_then(|v| v.as_str()) {
                return truncate_string(cmd, 50);
            }
            if let Some(pattern) = json.get("pattern").and_then(|v| v.as_str()) {
                return format!("\"{}\"", truncate_string(pattern, 30));
            }
        }
    }

    String::new()
}

fn build_action_result(event: &AgentEventV1) -> ActionResult {
    match event.tool_status {
        Some(ToolStatus::Success) => {
            if let Some(exit_code) = event.tool_exit_code {
                if exit_code != 0 {
                    return ActionResult::Failure {
                        error: format!("exit code {}", exit_code),
                    };
                }
            }
            ActionResult::Success {
                summary: String::new(),
            }
        }
        Some(ToolStatus::Error) => {
            let error = event
                .text
                .as_ref()
                .map(|t| truncate_string(t, 100))
                .unwrap_or_else(|| "unknown error".to_string());
            ActionResult::Failure { error }
        }
        _ => ActionResult::Missing,
    }
}

fn determine_outcome(chain: &[ChainItem]) -> TurnOutcome {
    for item in chain.iter().rev() {
        if let ChainItem::Action { result, .. } = item {
            if matches!(result, ActionResult::Interrupted) {
                return TurnOutcome::Interrupted {
                    reason: "user interrupted".to_string(),
                };
            }
        }
    }

    TurnOutcome::Completed
}

fn calculate_duration(start_ts: &Option<String>, end_ts: &Option<String>) -> Option<u64> {
    use chrono::DateTime;

    if let (Some(start), Some(end)) = (start_ts, end_ts) {
        if let (Ok(start_dt), Ok(end_dt)) = (
            DateTime::parse_from_rfc3339(start),
            DateTime::parse_from_rfc3339(end),
        ) {
            let duration = end_dt.signed_duration_since(start_dt);
            return Some(duration.num_milliseconds().max(0) as u64);
        }
    }
    None
}

fn infer_system_message_kind(message: &str) -> SystemMessageKind {
    let lower = message.to_lowercase();
    if lower.contains("error") || lower.contains("failed") {
        SystemMessageKind::Error
    } else if lower.contains("warning") || lower.contains("warn") {
        SystemMessageKind::Warning
    } else {
        SystemMessageKind::Info
    }
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let chars: Vec<char> = s.chars().take(max_len - 3).collect();
        format!("{}...", chars.iter().collect::<String>())
    }
}
