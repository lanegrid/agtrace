use super::stats::{calculate_turn_stats, merge_usage};
use super::step_builder::StepBuilder;
use super::types::*;
use agtrace_types::{AgentEvent, EventPayload};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use uuid::Uuid;

pub struct TurnBuilder {
    id: Uuid,
    timestamp: DateTime<Utc>,
    user: UserMessage,

    steps: Vec<StepBuilder>,
    current_step: StepBuilder,

    pending_calls: HashMap<Uuid, (usize, usize)>,
}

impl TurnBuilder {
    pub fn new(id: Uuid, timestamp: DateTime<Utc>, user: UserMessage) -> Self {
        Self {
            id,
            timestamp,
            user,
            steps: Vec::new(),
            current_step: StepBuilder::new(timestamp),
            pending_calls: HashMap::new(),
        }
    }

    pub fn add_event(&mut self, event: &AgentEvent) {
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

                let already_exists = self
                    .current_step
                    .tool_executions
                    .iter()
                    .any(|t| t.call.event_id == event.id);

                if already_exists {
                    return;
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
                if !self.current_step.is_empty() {
                    merge_usage(&mut self.current_step.usage, usage);
                } else if let Some(last_step) = self.steps.last_mut() {
                    merge_usage(&mut last_step.usage, usage);
                } else {
                    merge_usage(&mut self.current_step.usage, usage);
                }
            }

            EventPayload::Notification(_) => {}

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

    pub fn build(mut self) -> Option<AgentTurn> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use agtrace_types::{MessagePayload, ReasoningPayload, StreamId};

    #[test]
    fn test_turn_builder_basic() {
        let timestamp = Utc::now();
        let user_id = Uuid::new_v4();
        let user = UserMessage {
            event_id: user_id,
            content: agtrace_types::UserPayload {
                text: "Hello".to_string(),
            },
        };

        let builder = TurnBuilder::new(user_id, timestamp, user.clone());
        let turn = builder.build();

        assert!(turn.is_none());
    }

    #[test]
    fn test_turn_builder_with_message() {
        let timestamp = Utc::now();
        let user_id = Uuid::new_v4();
        let user = UserMessage {
            event_id: user_id,
            content: agtrace_types::UserPayload {
                text: "Hello".to_string(),
            },
        };

        let mut builder = TurnBuilder::new(user_id, timestamp, user.clone());

        let msg_event = AgentEvent {
            id: Uuid::new_v4(),
            session_id: user_id,
            parent_id: None,
            stream_id: StreamId::Main,
            timestamp,
            metadata: None,
            payload: EventPayload::Message(MessagePayload {
                text: "Response".to_string(),
            }),
        };

        builder.add_event(&msg_event);

        let turn = builder.build().unwrap();
        assert_eq!(turn.steps.len(), 1);
        assert!(turn.steps[0].message.is_some());
    }

    #[test]
    fn test_turn_builder_reasoning_creates_new_step() {
        let timestamp = Utc::now();
        let user_id = Uuid::new_v4();
        let user = UserMessage {
            event_id: user_id,
            content: agtrace_types::UserPayload {
                text: "Hello".to_string(),
            },
        };

        let mut builder = TurnBuilder::new(user_id, timestamp, user.clone());

        let reasoning1 = AgentEvent {
            id: Uuid::new_v4(),
            session_id: user_id,
            parent_id: None,
            stream_id: StreamId::Main,
            timestamp,
            metadata: None,
            payload: EventPayload::Reasoning(ReasoningPayload {
                text: "Thinking 1".to_string(),
            }),
        };

        let reasoning2 = AgentEvent {
            id: Uuid::new_v4(),
            session_id: user_id,
            parent_id: None,
            stream_id: StreamId::Main,
            timestamp,
            metadata: None,
            payload: EventPayload::Reasoning(ReasoningPayload {
                text: "Thinking 2".to_string(),
            }),
        };

        builder.add_event(&reasoning1);
        builder.add_event(&reasoning2);

        let turn = builder.build().unwrap();
        assert_eq!(turn.steps.len(), 2);
    }
}
