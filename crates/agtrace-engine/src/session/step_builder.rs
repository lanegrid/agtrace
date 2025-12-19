use super::types::*;
use chrono::{DateTime, Utc};
use uuid::Uuid;

pub struct StepBuilder {
    pub id: Option<Uuid>,
    pub timestamp: DateTime<Utc>,
    pub reasoning: Option<ReasoningBlock>,
    pub message: Option<MessageBlock>,
    pub tool_executions: Vec<ToolExecution>,
    pub usage: Option<agtrace_types::TokenUsagePayload>,
}

impl StepBuilder {
    pub fn new(timestamp: DateTime<Utc>) -> Self {
        Self {
            id: None,
            timestamp,
            reasoning: None,
            message: None,
            tool_executions: Vec::new(),
            usage: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.reasoning.is_none()
            && self.message.is_none()
            && self.tool_executions.is_empty()
            && self.usage.is_none()
    }

    pub fn build(self) -> AgentStep {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_step_builder_empty() {
        let timestamp = Utc::now();
        let builder = StepBuilder::new(timestamp);
        assert!(builder.is_empty());
    }

    #[test]
    fn test_step_builder_with_message() {
        let timestamp = Utc::now();
        let mut builder = StepBuilder::new(timestamp);
        builder.message = Some(MessageBlock {
            event_id: Uuid::new_v4(),
            content: agtrace_types::MessagePayload {
                text: "test".to_string(),
            },
        });
        assert!(!builder.is_empty());
    }

    #[test]
    fn test_step_builder_build() {
        let timestamp = Utc::now();
        let mut builder = StepBuilder::new(timestamp);
        let event_id = Uuid::new_v4();
        builder.id = Some(event_id);
        builder.message = Some(MessageBlock {
            event_id,
            content: agtrace_types::MessagePayload {
                text: "test".to_string(),
            },
        });

        let step = builder.build();
        assert_eq!(step.id, event_id);
        assert_eq!(step.timestamp, timestamp);
        assert!(!step.is_failed);
    }

    #[test]
    fn test_step_builder_build_with_error() {
        let timestamp = Utc::now();
        let mut builder = StepBuilder::new(timestamp);

        let call_event_id = Uuid::new_v4();
        builder.tool_executions.push(ToolExecution {
            call: ToolCallBlock {
                event_id: call_event_id,
                timestamp,
                provider_call_id: Some("call-1".to_string()),
                content: agtrace_types::ToolCallPayload {
                    provider_call_id: Some("call-1".to_string()),
                    name: "test".to_string(),
                    arguments: serde_json::json!({}),
                },
            },
            result: None,
            duration_ms: None,
            is_error: true,
        });

        let step = builder.build();
        assert!(step.is_failed);
    }
}
