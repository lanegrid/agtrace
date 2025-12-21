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
        let status = self.determine_status();

        AgentStep {
            id,
            timestamp: self.timestamp,
            reasoning: self.reasoning,
            message: self.message,
            tools: self.tool_executions,
            usage: self.usage,
            is_failed,
            status,
        }
    }

    fn determine_status(&self) -> super::types::StepStatus {
        use super::types::StepStatus;

        // 1. Error check (highest priority)
        if self.tool_executions.iter().any(|t| t.is_error) {
            return StepStatus::Failed;
        }

        // 2. Tool execution status (highest priority for completion)
        if !self.tool_executions.is_empty() {
            // If any tool is missing result, step is in progress
            if self.tool_executions.iter().any(|t| t.result.is_none()) {
                return StepStatus::InProgress;
            }
            // All tools have results -> Done
            return StepStatus::Done;
        }

        // 3. No tools: check message
        if self.message.is_some() {
            return StepStatus::Done;
        }

        // 4. In progress: reasoning only (waiting for next action)
        if self.reasoning.is_some() {
            return StepStatus::InProgress;
        }

        // Default: Done (safe side)
        StepStatus::Done
    }
}

#[cfg(test)]
mod tests {
    use super::super::types::StepStatus;
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

    // ========================================
    // Step Status Determination Tests
    // ========================================

    #[test]
    fn test_status_done_with_message() {
        let timestamp = Utc::now();
        let mut builder = StepBuilder::new(timestamp);
        builder.message = Some(MessageBlock {
            event_id: Uuid::new_v4(),
            content: agtrace_types::MessagePayload {
                text: "Response".to_string(),
            },
        });

        let step = builder.build();
        assert_eq!(step.status, StepStatus::Done);
    }

    #[test]
    fn test_status_done_with_reasoning_and_message() {
        let timestamp = Utc::now();
        let mut builder = StepBuilder::new(timestamp);
        builder.reasoning = Some(ReasoningBlock {
            event_id: Uuid::new_v4(),
            content: agtrace_types::ReasoningPayload {
                text: "Thinking...".to_string(),
            },
        });
        builder.message = Some(MessageBlock {
            event_id: Uuid::new_v4(),
            content: agtrace_types::MessagePayload {
                text: "Response".to_string(),
            },
        });

        let step = builder.build();
        assert_eq!(step.status, StepStatus::Done);
    }

    #[test]
    fn test_status_in_progress_reasoning_only() {
        let timestamp = Utc::now();
        let mut builder = StepBuilder::new(timestamp);
        builder.reasoning = Some(ReasoningBlock {
            event_id: Uuid::new_v4(),
            content: agtrace_types::ReasoningPayload {
                text: "Thinking...".to_string(),
            },
        });

        let step = builder.build();
        assert_eq!(step.status, StepStatus::InProgress);
    }

    #[test]
    fn test_status_in_progress_tool_without_result() {
        let timestamp = Utc::now();
        let mut builder = StepBuilder::new(timestamp);

        let call_event_id = Uuid::new_v4();
        builder.tool_executions.push(ToolExecution {
            call: ToolCallBlock {
                event_id: call_event_id,
                timestamp,
                provider_call_id: Some("toolu_123".to_string()),
                content: agtrace_types::ToolCallPayload {
                    provider_call_id: Some("toolu_123".to_string()),
                    name: "bash".to_string(),
                    arguments: serde_json::json!({"command": "ls"}),
                },
            },
            result: None,
            duration_ms: None,
            is_error: false,
        });

        let step = builder.build();
        assert_eq!(step.status, StepStatus::InProgress);
    }

    #[test]
    fn test_status_done_tool_with_result() {
        let timestamp = Utc::now();
        let mut builder = StepBuilder::new(timestamp);

        let call_event_id = Uuid::new_v4();
        let result_event_id = Uuid::new_v4();

        builder.tool_executions.push(ToolExecution {
            call: ToolCallBlock {
                event_id: call_event_id,
                timestamp,
                provider_call_id: Some("toolu_123".to_string()),
                content: agtrace_types::ToolCallPayload {
                    provider_call_id: Some("toolu_123".to_string()),
                    name: "bash".to_string(),
                    arguments: serde_json::json!({"command": "ls"}),
                },
            },
            result: Some(ToolResultBlock {
                event_id: result_event_id,
                timestamp,
                tool_call_id: call_event_id,
                content: agtrace_types::ToolResultPayload {
                    output: "file1.txt\nfile2.txt".to_string(),
                    tool_call_id: call_event_id,
                    is_error: false,
                },
            }),
            duration_ms: Some(100),
            is_error: false,
        });

        let step = builder.build();
        assert_eq!(step.status, StepStatus::Done);
    }

    #[test]
    fn test_status_failed_with_tool_error() {
        let timestamp = Utc::now();
        let mut builder = StepBuilder::new(timestamp);

        let call_event_id = Uuid::new_v4();
        let result_event_id = Uuid::new_v4();

        builder.tool_executions.push(ToolExecution {
            call: ToolCallBlock {
                event_id: call_event_id,
                timestamp,
                provider_call_id: Some("toolu_123".to_string()),
                content: agtrace_types::ToolCallPayload {
                    provider_call_id: Some("toolu_123".to_string()),
                    name: "bash".to_string(),
                    arguments: serde_json::json!({"command": "invalid"}),
                },
            },
            result: Some(ToolResultBlock {
                event_id: result_event_id,
                timestamp,
                tool_call_id: call_event_id,
                content: agtrace_types::ToolResultPayload {
                    output: "command not found".to_string(),
                    tool_call_id: call_event_id,
                    is_error: true,
                },
            }),
            duration_ms: Some(50),
            is_error: true,
        });

        let step = builder.build();
        assert_eq!(step.status, StepStatus::Failed);
        assert!(step.is_failed);
    }

    #[test]
    fn test_status_in_progress_mixed_tools() {
        let timestamp = Utc::now();
        let mut builder = StepBuilder::new(timestamp);

        let call1_id = Uuid::new_v4();
        let call2_id = Uuid::new_v4();

        // First tool: completed
        builder.tool_executions.push(ToolExecution {
            call: ToolCallBlock {
                event_id: call1_id,
                timestamp,
                provider_call_id: Some("toolu_1".to_string()),
                content: agtrace_types::ToolCallPayload {
                    provider_call_id: Some("toolu_1".to_string()),
                    name: "read".to_string(),
                    arguments: serde_json::json!({}),
                },
            },
            result: Some(ToolResultBlock {
                event_id: Uuid::new_v4(),
                timestamp,
                tool_call_id: call1_id,
                content: agtrace_types::ToolResultPayload {
                    output: "content".to_string(),
                    tool_call_id: call1_id,
                    is_error: false,
                },
            }),
            duration_ms: Some(100),
            is_error: false,
        });

        // Second tool: pending
        builder.tool_executions.push(ToolExecution {
            call: ToolCallBlock {
                event_id: call2_id,
                timestamp,
                provider_call_id: Some("toolu_2".to_string()),
                content: agtrace_types::ToolCallPayload {
                    provider_call_id: Some("toolu_2".to_string()),
                    name: "bash".to_string(),
                    arguments: serde_json::json!({}),
                },
            },
            result: None,
            duration_ms: None,
            is_error: false,
        });

        let step = builder.build();
        assert_eq!(step.status, StepStatus::InProgress);
    }

    #[test]
    fn test_status_message_then_tool_without_result() {
        // Claude Code pattern: thinking -> text -> tool_use
        // This should be InProgress because tool has no result
        let timestamp = Utc::now();
        let mut builder = StepBuilder::new(timestamp);

        builder.reasoning = Some(ReasoningBlock {
            event_id: Uuid::new_v4(),
            content: agtrace_types::ReasoningPayload {
                text: "Thinking...".to_string(),
            },
        });

        builder.message = Some(MessageBlock {
            event_id: Uuid::new_v4(),
            content: agtrace_types::MessagePayload {
                text: "Let me check that".to_string(),
            },
        });

        let tool_id = Uuid::new_v4();
        builder.tool_executions.push(ToolExecution {
            call: ToolCallBlock {
                event_id: tool_id,
                timestamp,
                provider_call_id: Some("toolu_123".to_string()),
                content: agtrace_types::ToolCallPayload {
                    provider_call_id: Some("toolu_123".to_string()),
                    name: "bash".to_string(),
                    arguments: serde_json::json!({"command": "ls"}),
                },
            },
            result: None,
            duration_ms: None,
            is_error: false,
        });

        let step = builder.build();
        assert_eq!(
            step.status,
            StepStatus::InProgress,
            "Step with message + tool without result should be InProgress"
        );
    }

    #[test]
    fn test_status_message_then_tool_with_result() {
        // Claude Code pattern: thinking -> text -> tool_use (with result)
        // This should be Done because all tools have results
        let timestamp = Utc::now();
        let mut builder = StepBuilder::new(timestamp);

        builder.reasoning = Some(ReasoningBlock {
            event_id: Uuid::new_v4(),
            content: agtrace_types::ReasoningPayload {
                text: "Thinking...".to_string(),
            },
        });

        builder.message = Some(MessageBlock {
            event_id: Uuid::new_v4(),
            content: agtrace_types::MessagePayload {
                text: "Let me check that".to_string(),
            },
        });

        let tool_id = Uuid::new_v4();
        builder.tool_executions.push(ToolExecution {
            call: ToolCallBlock {
                event_id: tool_id,
                timestamp,
                provider_call_id: Some("toolu_123".to_string()),
                content: agtrace_types::ToolCallPayload {
                    provider_call_id: Some("toolu_123".to_string()),
                    name: "bash".to_string(),
                    arguments: serde_json::json!({"command": "ls"}),
                },
            },
            result: Some(ToolResultBlock {
                event_id: Uuid::new_v4(),
                timestamp,
                tool_call_id: tool_id,
                content: agtrace_types::ToolResultPayload {
                    output: "file.txt".to_string(),
                    tool_call_id: tool_id,
                    is_error: false,
                },
            }),
            duration_ms: Some(100),
            is_error: false,
        });

        let step = builder.build();
        assert_eq!(
            step.status,
            StepStatus::Done,
            "Step with message + tool with result should be Done"
        );
    }
}
