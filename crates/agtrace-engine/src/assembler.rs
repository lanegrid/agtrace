use crate::session::*;
use agtrace_types::v2::{AgentEvent, EventPayload};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::mem;
use uuid::Uuid;

pub fn assemble_session(events: &[AgentEvent]) -> Option<AgentSession> {
    if events.is_empty() {
        return None;
    }

    let session_id = events.first()?.trace_id;
    let start_time = events.first()?.timestamp;
    let end_time = events.last().map(|e| e.timestamp);

    let turns = build_turns(events);

    let stats = calculate_session_stats(&turns, start_time, end_time);

    Some(AgentSession {
        session_id,
        start_time,
        end_time,
        turns,
        stats,
    })
}

fn build_turns(events: &[AgentEvent]) -> Vec<AgentTurn> {
    let mut turns = Vec::new();
    let mut current_turn: Option<TurnBuilder> = None;

    for event in events {
        match &event.payload {
            EventPayload::User(user) => {
                if let Some(builder) = current_turn.take() {
                    if let Some(turn) = builder.build() {
                        turns.push(turn);
                    }
                }

                current_turn = Some(TurnBuilder::new(
                    event.id, // Use user event ID as turn ID
                    event.timestamp,
                    UserMessage {
                        event_id: event.id,
                        content: user.clone(),
                    },
                ));
            }
            _ => {
                if let Some(ref mut builder) = current_turn {
                    builder.add_event(event);
                }
            }
        }
    }

    if let Some(builder) = current_turn {
        if let Some(turn) = builder.build() {
            turns.push(turn);
        }
    }

    turns
}

struct TurnBuilder {
    id: Uuid,
    timestamp: DateTime<Utc>,
    user: UserMessage,
    step_builder: StepBuilder,
    completed_steps: Vec<AgentStep>,
}

impl TurnBuilder {
    fn new(id: Uuid, timestamp: DateTime<Utc>, user: UserMessage) -> Self {
        Self {
            id,
            timestamp,
            user,
            step_builder: StepBuilder::new(timestamp),
            completed_steps: Vec::new(),
        }
    }

    fn add_event(&mut self, event: &AgentEvent) {
        match &event.payload {
            EventPayload::TokenUsage(usage) => {
                self.step_builder.usage = Some(usage.clone());
                let builder =
                    mem::replace(&mut self.step_builder, StepBuilder::new(event.timestamp));
                if let Some(step) = builder.build() {
                    self.completed_steps.push(step);
                }
            }
            EventPayload::Reasoning(reasoning) => {
                self.step_builder.set_id_if_none(event.id);
                self.step_builder.reasoning = Some(ReasoningBlock {
                    event_id: event.id,
                    content: reasoning.clone(),
                });
            }
            EventPayload::Message(message) => {
                self.step_builder.set_id_if_none(event.id);
                self.step_builder.message = Some(MessageBlock {
                    event_id: event.id,
                    content: message.clone(),
                });
            }
            EventPayload::ToolCall(tool_call) => {
                self.step_builder.set_id_if_none(event.id);
                self.step_builder.tool_calls.push((
                    event.id,
                    event.timestamp,
                    tool_call.provider_call_id.clone(),
                    tool_call.clone(),
                ));
            }
            EventPayload::ToolResult(tool_result) => {
                self.step_builder.set_id_if_none(event.id);
                self.step_builder.tool_results.push((
                    event.id,
                    event.timestamp,
                    tool_result.tool_call_id,
                    tool_result.clone(),
                ));
            }
            EventPayload::User(_) => {}
        }
    }

    fn build(mut self) -> Option<AgentTurn> {
        if let Some(step) = self.step_builder.build() {
            self.completed_steps.push(step);
        }

        if self.completed_steps.is_empty() {
            return None;
        }

        let stats = calculate_turn_stats(&self.completed_steps, self.timestamp);

        Some(AgentTurn {
            id: self.id,
            timestamp: self.timestamp,
            user: self.user,
            steps: self.completed_steps,
            stats,
        })
    }
}

struct StepBuilder {
    id: Option<Uuid>, // None until first event is added
    timestamp: DateTime<Utc>,
    reasoning: Option<ReasoningBlock>,
    message: Option<MessageBlock>,
    tool_calls: Vec<(
        Uuid,
        DateTime<Utc>,
        Option<String>,
        agtrace_types::v2::ToolCallPayload,
    )>,
    tool_results: Vec<(
        Uuid,
        DateTime<Utc>,
        Uuid,
        agtrace_types::v2::ToolResultPayload,
    )>,
    usage: Option<agtrace_types::v2::TokenUsagePayload>,
}

impl StepBuilder {
    fn new(timestamp: DateTime<Utc>) -> Self {
        Self {
            id: None,
            timestamp,
            reasoning: None,
            message: None,
            tool_calls: Vec::new(),
            tool_results: Vec::new(),
            usage: None,
        }
    }

    fn set_id_if_none(&mut self, id: Uuid) {
        if self.id.is_none() {
            self.id = Some(id);
        }
    }

    fn build(self) -> Option<AgentStep> {
        if self.reasoning.is_none()
            && self.message.is_none()
            && self.tool_calls.is_empty()
            && self.usage.is_none()
        {
            return None;
        }

        // Use first event ID, or generate one if somehow none was set
        let id = self.id.unwrap_or_else(Uuid::new_v4);

        let tools = build_tool_executions(&self.tool_calls, &self.tool_results);

        let is_failed = tools.iter().any(|t| t.is_error)
            || self.tool_results.iter().any(|(_, _, _, r)| r.is_error);

        Some(AgentStep {
            id,
            timestamp: self.timestamp,
            reasoning: self.reasoning,
            message: self.message,
            tools,
            usage: self.usage,
            is_failed,
        })
    }
}

fn build_tool_executions(
    tool_calls: &[(
        Uuid,
        DateTime<Utc>,
        Option<String>,
        agtrace_types::v2::ToolCallPayload,
    )],
    tool_results: &[(
        Uuid,
        DateTime<Utc>,
        Uuid,
        agtrace_types::v2::ToolResultPayload,
    )],
) -> Vec<ToolExecution> {
    let mut results_map: HashMap<
        Uuid,
        (Uuid, DateTime<Utc>, agtrace_types::v2::ToolResultPayload),
    > = HashMap::new();

    for (result_event_id, result_timestamp, tool_call_id, result) in tool_results {
        results_map.insert(
            *tool_call_id,
            (*result_event_id, *result_timestamp, result.clone()),
        );
    }

    tool_calls
        .iter()
        .map(|(call_event_id, call_timestamp, provider_call_id, call)| {
            let result = results_map.get(call_event_id).map(
                |(result_event_id, result_timestamp, result_payload)| ToolResultBlock {
                    event_id: *result_event_id,
                    timestamp: *result_timestamp,
                    tool_call_id: *call_event_id,
                    content: result_payload.clone(),
                },
            );

            let duration_ms = result
                .as_ref()
                .map(|r| (r.timestamp - *call_timestamp).num_milliseconds());

            let is_error = result.as_ref().map(|r| r.content.is_error).unwrap_or(false);

            ToolExecution {
                call: ToolCallBlock {
                    event_id: *call_event_id,
                    timestamp: *call_timestamp,
                    provider_call_id: provider_call_id.clone(),
                    content: call.clone(),
                },
                result,
                duration_ms,
                is_error,
            }
        })
        .collect()
}

fn calculate_session_stats(
    turns: &[AgentTurn],
    start_time: DateTime<Utc>,
    end_time: Option<DateTime<Utc>>,
) -> SessionStats {
    let total_turns = turns.len();
    let duration_seconds = end_time
        .map(|end| (end - start_time).num_seconds())
        .unwrap_or(0);
    let total_tokens: i64 = turns.iter().map(|t| t.stats.total_tokens as i64).sum();

    SessionStats {
        total_turns,
        duration_seconds,
        total_tokens,
    }
}

fn calculate_turn_stats(steps: &[AgentStep], turn_start: DateTime<Utc>) -> TurnStats {
    let step_count = steps.len();
    let duration_ms = steps
        .last()
        .map(|last| (last.timestamp - turn_start).num_milliseconds())
        .unwrap_or(0);
    let total_tokens: i32 = steps
        .iter()
        .filter_map(|s| s.usage.as_ref())
        .map(|u| u.total_tokens)
        .sum();

    TurnStats {
        duration_ms,
        step_count,
        total_tokens,
    }
}
