use crate::session::*;
use agtrace_types::v2::{AgentEvent, EventPayload};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
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
                    event.id,
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

    steps: Vec<StepBuilder>,
    current_step: StepBuilder,

    // Key: Tool Call Event ID, Value: (Step Index, Call Index in Step)
    pending_calls: HashMap<Uuid, (usize, usize)>,
}

impl TurnBuilder {
    fn new(id: Uuid, timestamp: DateTime<Utc>, user: UserMessage) -> Self {
        Self {
            id,
            timestamp,
            user,
            steps: Vec::new(),
            current_step: StepBuilder::new(timestamp),
            pending_calls: HashMap::new(),
        }
    }

    fn add_event(&mut self, event: &AgentEvent) {
        match &event.payload {
            EventPayload::Reasoning(reasoning) => {
                self.ensure_new_step_if_needed(event.timestamp);

                self.current_step.id = Some(event.id);
                self.current_step.reasoning = Some(ReasoningBlock {
                    event_id: event.id,
                    content: reasoning.clone(),
                });
            }

            EventPayload::Message(message) => {
                if self.current_step.message.is_some() {
                    self.start_new_step(event.timestamp);
                }

                if self.current_step.id.is_none() {
                    self.current_step.id = Some(event.id);
                }

                self.current_step.message = Some(MessageBlock {
                    event_id: event.id,
                    content: message.clone(),
                });
            }

            EventPayload::ToolCall(tool_call) => {
                if self.current_step.id.is_none() {
                    self.current_step.id = Some(event.id);
                }

                let call_block = ToolCallBlock {
                    event_id: event.id,
                    timestamp: event.timestamp,
                    provider_call_id: tool_call.provider_call_id.clone(),
                    content: tool_call.clone(),
                };

                let call_idx = self.current_step.tool_executions.len();
                self.current_step.tool_executions.push(ToolExecution {
                    call: call_block,
                    result: None,
                    duration_ms: None,
                    is_error: false,
                });

                self.pending_calls
                    .insert(event.id, (self.steps.len(), call_idx));
            }

            EventPayload::ToolResult(tool_result) => {
                let result_block = ToolResultBlock {
                    event_id: event.id,
                    timestamp: event.timestamp,
                    tool_call_id: tool_result.tool_call_id,
                    content: tool_result.clone(),
                };

                if let Some(&(step_idx, call_idx)) =
                    self.pending_calls.get(&tool_result.tool_call_id)
                {
                    let target_step = if step_idx < self.steps.len() {
                        &mut self.steps[step_idx]
                    } else {
                        &mut self.current_step
                    };

                    if let Some(exec) = target_step.tool_executions.get_mut(call_idx) {
                        let duration = (event.timestamp - exec.call.timestamp).num_milliseconds();

                        exec.result = Some(result_block);
                        exec.duration_ms = Some(duration);
                        exec.is_error = tool_result.is_error;
                    }

                    self.pending_calls.remove(&tool_result.tool_call_id);
                }
            }

            EventPayload::TokenUsage(usage) => {
                if let Some(current) = &mut self.current_step.usage {
                    current.input_tokens += usage.input_tokens;
                    current.output_tokens += usage.output_tokens;
                    current.total_tokens += usage.total_tokens;
                } else {
                    self.current_step.usage = Some(usage.clone());
                }
            }

            EventPayload::User(_) => unreachable!(),
        }
    }

    fn ensure_new_step_if_needed(&mut self, timestamp: DateTime<Utc>) {
        if self.current_step.reasoning.is_some() {
            self.start_new_step(timestamp);
        }
    }

    fn start_new_step(&mut self, timestamp: DateTime<Utc>) {
        if self.current_step.is_empty() {
            return;
        }

        let completed = std::mem::replace(&mut self.current_step, StepBuilder::new(timestamp));
        self.steps.push(completed);
    }

    fn build(mut self) -> Option<AgentTurn> {
        if !self.current_step.is_empty() {
            self.steps.push(self.current_step);
        }

        if self.steps.is_empty() {
            return None;
        }

        let completed_steps: Vec<AgentStep> = self.steps.into_iter().map(|b| b.build()).collect();

        let stats = calculate_turn_stats(&completed_steps, self.timestamp);

        Some(AgentTurn {
            id: self.id,
            timestamp: self.timestamp,
            user: self.user,
            steps: completed_steps,
            stats,
        })
    }
}

struct StepBuilder {
    id: Option<Uuid>,
    timestamp: DateTime<Utc>,
    reasoning: Option<ReasoningBlock>,
    message: Option<MessageBlock>,
    tool_executions: Vec<ToolExecution>,
    usage: Option<agtrace_types::v2::TokenUsagePayload>,
}

impl StepBuilder {
    fn new(timestamp: DateTime<Utc>) -> Self {
        Self {
            id: None,
            timestamp,
            reasoning: None,
            message: None,
            tool_executions: Vec::new(),
            usage: None,
        }
    }

    fn is_empty(&self) -> bool {
        self.reasoning.is_none()
            && self.message.is_none()
            && self.tool_executions.is_empty()
            && self.usage.is_none()
    }

    fn build(self) -> AgentStep {
        let id = self.id.unwrap_or_else(Uuid::new_v4);

        let is_failed = self.tool_executions.iter().any(|t| t.is_error);

        AgentStep {
            id,
            timestamp: self.timestamp,
            reasoning: self.reasoning,
            message: self.message,
            tools: self.tool_executions,
            usage: self.usage,
            is_failed,
        }
    }
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
