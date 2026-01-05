use agtrace_sdk::types::{truncate, AgentEvent};
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::mcp::models::common::{EventType, Provider};

// ============================================================================
// Search Events API
// ============================================================================

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SearchEventsArgs {
    pub query: String,
    pub session_id: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub cursor: Option<String>,
    pub provider: Option<Provider>,
    pub event_type: Option<EventType>,
    pub project_root: Option<String>,
    pub project_hash: Option<String>,
}

impl SearchEventsArgs {
    pub fn limit(&self) -> usize {
        self.limit.unwrap_or(20).min(50)
    }
}

#[derive(Debug, Serialize)]
pub struct SearchEventsResponse {
    pub matches: Vec<EventMatch>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct EventMatch {
    pub session_id: String,
    pub event_index: usize,
    pub turn_index: usize,
    pub step_index: usize,
    pub event_type: EventType,
    pub preview: String,
    pub timestamp: DateTime<Utc>,
}

impl EventMatch {
    pub fn new(
        session_id: String,
        event_index: usize,
        turn_index: usize,
        step_index: usize,
        event: &AgentEvent,
    ) -> Self {
        let event_type = EventType::from_payload(&event.payload);
        let preview = Self::extract_preview(&event.payload);

        Self {
            session_id,
            event_index,
            turn_index,
            step_index,
            event_type,
            preview,
            timestamp: event.timestamp,
        }
    }

    fn extract_preview(payload: &agtrace_sdk::types::EventPayload) -> String {
        use agtrace_sdk::types::EventPayload;

        let text = match payload {
            EventPayload::ToolCall(tc) => {
                serde_json::to_string(tc).unwrap_or_else(|_| String::new())
            }
            EventPayload::ToolResult(tr) => {
                serde_json::to_string(tr).unwrap_or_else(|_| String::new())
            }
            EventPayload::User(u) => u.text.clone(),
            EventPayload::Message(m) => m.text.clone(),
            EventPayload::Reasoning(r) => r.text.clone(),
            EventPayload::TokenUsage(tu) => {
                format!("tokens: in={} out={}", tu.input.total(), tu.output.total())
            }
            EventPayload::Notification(n) => {
                serde_json::to_string(n).unwrap_or_else(|_| String::new())
            }
        };

        if text.len() > 200 {
            let truncated: String = text.chars().take(200).collect();
            format!("{}...", truncated)
        } else {
            text
        }
    }
}

// ============================================================================
// List Turns API
// ============================================================================

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListTurnsArgs {
    pub session_id: String,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub cursor: Option<String>,
}

impl ListTurnsArgs {
    pub fn limit(&self) -> usize {
        self.limit.unwrap_or(50).min(100)
    }
}

#[derive(Debug, Serialize)]
pub struct ListTurnsResponse {
    pub session_id: String,
    pub total_turns: usize,
    pub turns: Vec<TurnMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TurnMetadata {
    pub turn_index: usize,
    pub user_content: String,
    pub status: TurnStatus,
    pub step_count: usize,
    pub duration_ms: u64,
    pub total_tokens: u64,
    pub tools_used: HashMap<String, usize>,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TurnStatus {
    Completed,
    Failed,
}

impl ListTurnsResponse {
    pub fn new(
        session: agtrace_sdk::types::AgentSession,
        offset: usize,
        limit: usize,
        next_cursor: Option<String>,
    ) -> Self {
        let total_turns = session.turns.len();
        let turns: Vec<_> = session
            .turns
            .into_iter()
            .enumerate()
            .skip(offset)
            .take(limit)
            .map(|(idx, turn)| {
                let step_count = turn.steps.len();
                let duration_ms = turn.stats.duration_ms as u64;
                let total_tokens = turn.stats.total_tokens as u64;

                let mut tools_used: HashMap<String, usize> = HashMap::new();
                for step in &turn.steps {
                    for tool in &step.tools {
                        *tools_used
                            .entry(tool.call.content.name().to_string())
                            .or_insert(0) += 1;
                    }
                }

                let status = if turn
                    .steps
                    .iter()
                    .any(|s| s.tools.iter().any(|t| t.is_error))
                {
                    TurnStatus::Failed
                } else {
                    TurnStatus::Completed
                };

                let user_content = truncate(&turn.user.content.text, 100);

                TurnMetadata {
                    turn_index: idx,
                    user_content,
                    status,
                    step_count,
                    duration_ms,
                    total_tokens,
                    tools_used,
                }
            })
            .collect();

        Self {
            session_id: session.session_id.to_string(),
            total_turns,
            turns,
            next_cursor,
        }
    }
}

// ============================================================================
// Get Turns API
// ============================================================================

// Data-driven defaults based on actual distribution analysis:
// - 3,000 chars covers P90 (2,552 bytes) without truncation for most events
// - 30 steps covers P50 (15) comfortably and approaches P75 (38), allowing ~70% of turns to be read fully
// Worst case: 3,000 × 30 = 90k chars ≈ 22k tokens (still safe for 1-2 turns per request)
// Average case: 1,176 × 30 ≈ 35k chars ≈ 8k tokens (very safe)
const DEFAULT_MAX_CHARS_PER_FIELD: usize = 3_000;
const DEFAULT_MAX_STEPS_LIMIT: usize = 30;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetTurnsArgs {
    pub session_id: String,
    pub turn_indices: Vec<usize>,
    #[serde(default = "default_true")]
    pub truncate: Option<bool>,
    #[serde(default)]
    pub max_chars_per_field: Option<usize>,
    #[serde(default)]
    pub max_steps_limit: Option<usize>,
}

fn default_true() -> Option<bool> {
    Some(true)
}

impl GetTurnsArgs {
    pub fn should_truncate(&self) -> bool {
        self.truncate.unwrap_or(true)
    }

    pub fn max_chars(&self) -> usize {
        self.max_chars_per_field
            .unwrap_or(DEFAULT_MAX_CHARS_PER_FIELD)
    }

    pub fn max_steps(&self) -> usize {
        self.max_steps_limit.unwrap_or(DEFAULT_MAX_STEPS_LIMIT)
    }
}

#[derive(Debug, Serialize)]
pub struct GetTurnsResponse {
    pub turns: Vec<TurnDetail>,
}

#[derive(Debug, Serialize)]
pub struct TurnDetail {
    pub turn_index: usize,
    pub user_content: String,
    pub steps: Vec<StepDetail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steps_truncated: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct StepDetail {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    pub tools: Vec<ToolDetail>,
    pub is_failed: bool,
}

#[derive(Debug, Serialize)]
pub struct ToolDetail {
    pub name: String,
    pub args: String,
    pub result: Option<String>,
    pub is_error: bool,
}

impl GetTurnsResponse {
    pub fn new(
        session: agtrace_sdk::types::AgentSession,
        args: &GetTurnsArgs,
    ) -> Result<Self, String> {
        let should_truncate = args.should_truncate();
        let max_chars = args.max_chars();
        let max_steps = args.max_steps();

        let mut turns = Vec::new();

        for &turn_index in &args.turn_indices {
            if turn_index >= session.turns.len() {
                return Err(format!(
                    "Turn index {} out of range (session has {} turns)",
                    turn_index,
                    session.turns.len()
                ));
            }

            let turn = &session.turns[turn_index];
            let user_content = if should_truncate {
                truncate(&turn.user.content.text, max_chars)
            } else {
                turn.user.content.text.clone()
            };

            let total_steps = turn.steps.len();
            let steps_to_process = if should_truncate && total_steps > max_steps {
                &turn.steps[..max_steps]
            } else {
                &turn.steps[..]
            };

            let steps: Vec<StepDetail> = steps_to_process
                .iter()
                .map(|step| {
                    let reasoning = step.reasoning.as_ref().map(|r| {
                        if should_truncate {
                            truncate(&r.content.text, max_chars)
                        } else {
                            r.content.text.clone()
                        }
                    });

                    let message = step.message.as_ref().map(|m| {
                        if should_truncate {
                            truncate(&m.content.text, max_chars)
                        } else {
                            m.content.text.clone()
                        }
                    });

                    let tools: Vec<ToolDetail> = step
                        .tools
                        .iter()
                        .map(|tool| {
                            let args_json = serde_json::to_string(&tool.call.content)
                                .unwrap_or_else(|_| String::from("{}"));
                            let result_json = tool.result.as_ref().map(|r| {
                                serde_json::to_string(r).unwrap_or_else(|_| String::from("{}"))
                            });

                            ToolDetail {
                                name: tool.call.content.name().to_string(),
                                args: if should_truncate {
                                    truncate(&args_json, max_chars)
                                } else {
                                    args_json
                                },
                                result: result_json.map(|r| {
                                    if should_truncate {
                                        truncate(&r, max_chars)
                                    } else {
                                        r
                                    }
                                }),
                                is_error: tool.is_error,
                            }
                        })
                        .collect();

                    StepDetail {
                        reasoning,
                        message,
                        tools,
                        is_failed: step.is_failed,
                    }
                })
                .collect();

            let steps_truncated = if should_truncate && total_steps > max_steps {
                Some(true)
            } else {
                None
            };

            turns.push(TurnDetail {
                turn_index,
                user_content,
                steps,
                steps_truncated,
            });
        }

        Ok(Self { turns })
    }
}
