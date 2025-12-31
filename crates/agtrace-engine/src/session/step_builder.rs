use super::types::*;
use agtrace_types::ContextWindowUsage;
use chrono::{DateTime, Utc};
use uuid::Uuid;

pub struct StepBuilder {
    pub id: Option<Uuid>,
    pub timestamp: DateTime<Utc>,
    pub reasoning: Option<ReasoningBlock>,
    pub message: Option<MessageBlock>,
    pub tool_executions: Vec<ToolExecution>,
    pub usage: Option<ContextWindowUsage>,
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

    /// Determine step completion status based on content and tool execution state
    ///
    /// Status logic:
    /// - Failed: Any tool has error
    /// - InProgress: Tools exist but results incomplete, or reasoning-only waiting for action
    /// - Done: All content complete (message present, or all tools have results)
    fn determine_status(&self) -> super::types::StepStatus {
        use super::types::StepStatus;

        // Priority 1: Error check
        if self.tool_executions.iter().any(|t| t.is_error) {
            return StepStatus::Failed;
        }

        // Priority 2: Tool execution status
        // If tools exist, their completion determines status
        if !self.tool_executions.is_empty() {
            if self.tool_executions.iter().any(|t| t.result.is_none()) {
                return StepStatus::InProgress;
            }
            return StepStatus::Done;
        }

        // Priority 3: Content completion (no tools)
        // Message indicates completion
        if self.message.is_some() {
            return StepStatus::Done;
        }

        // Priority 4: Reasoning-only state
        // Still waiting for message or tools
        if self.reasoning.is_some() {
            return StepStatus::InProgress;
        }

        // Default: Empty or incomplete step
        StepStatus::Done
    }
}

#[cfg(test)]
mod tests {
    use super::super::types::StepStatus;
    use super::*;
    use agtrace_types::{ExecuteArgs, FileReadArgs, ToolCallPayload};

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
                content: ToolCallPayload::Execute {
                    name: "test".to_string(),
                    arguments: ExecuteArgs {
                        command: Some("test".to_string()),
                        description: None,
                        timeout: None,
                        extra: serde_json::json!({}),
                    },
                    provider_call_id: Some("call-1".to_string()),
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
                content: ToolCallPayload::Execute {
                    name: "bash".to_string(),
                    arguments: ExecuteArgs {
                        command: Some("ls".to_string()),
                        description: None,
                        timeout: None,
                        extra: serde_json::json!({}),
                    },
                    provider_call_id: Some("toolu_123".to_string()),
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
                content: ToolCallPayload::Execute {
                    name: "bash".to_string(),
                    arguments: ExecuteArgs {
                        command: Some("ls".to_string()),
                        description: None,
                        timeout: None,
                        extra: serde_json::json!({}),
                    },
                    provider_call_id: Some("toolu_123".to_string()),
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
                content: ToolCallPayload::Execute {
                    name: "bash".to_string(),
                    arguments: ExecuteArgs {
                        command: Some("invalid".to_string()),
                        description: None,
                        timeout: None,
                        extra: serde_json::json!({}),
                    },
                    provider_call_id: Some("toolu_123".to_string()),
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
                content: ToolCallPayload::FileRead {
                    name: "read".to_string(),
                    arguments: FileReadArgs {
                        file_path: None,
                        path: None,
                        pattern: None,
                        extra: serde_json::json!({}),
                    },
                    provider_call_id: Some("toolu_1".to_string()),
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
                content: ToolCallPayload::Execute {
                    name: "bash".to_string(),
                    arguments: ExecuteArgs {
                        command: None,
                        description: None,
                        timeout: None,
                        extra: serde_json::json!({}),
                    },
                    provider_call_id: Some("toolu_2".to_string()),
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
                content: ToolCallPayload::Execute {
                    name: "bash".to_string(),
                    arguments: ExecuteArgs {
                        command: Some("ls".to_string()),
                        description: None,
                        timeout: None,
                        extra: serde_json::json!({}),
                    },
                    provider_call_id: Some("toolu_123".to_string()),
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
                content: ToolCallPayload::Execute {
                    name: "bash".to_string(),
                    arguments: ExecuteArgs {
                        command: Some("ls".to_string()),
                        description: None,
                        timeout: None,
                        extra: serde_json::json!({}),
                    },
                    provider_call_id: Some("toolu_123".to_string()),
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
