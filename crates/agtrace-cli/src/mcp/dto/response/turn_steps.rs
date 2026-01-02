use agtrace_sdk::types::{AgentStep, AgentTurn, StepStatus};
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::mcp::dto::common::ResponseMeta;

/// Turn steps response for get_turn_steps tool
/// Target size: 20-50 KB (paginated if needed)
#[derive(Debug, Serialize)]
pub struct TurnStepsResponse {
    pub session_id: String,
    pub turn_index: usize,
    pub turn_id: String,
    pub timestamp: DateTime<Utc>,
    pub steps: Vec<StepDetailDto>,
    pub _meta: ResponseMeta,
}

#[derive(Debug, Serialize)]
pub struct StepDetailDto {
    pub step_index: usize,
    pub status: StepStatusDto,
    pub reasoning: Option<ReasoningDto>,
    pub tools: Vec<ToolExecutionDto>,
    pub message: Option<MessageDto>,
    pub tokens: Option<TokenUsageDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StepStatusDto {
    Done,
    InProgress,
    Failed,
}

#[derive(Debug, Serialize)]
pub struct ReasoningDto {
    pub text: String, // Truncated to 500 chars
}

#[derive(Debug, Serialize)]
pub struct ToolExecutionDto {
    pub name: String,
    pub input_summary: String,
    pub result_summary: String,
    pub is_error: bool,
}

#[derive(Debug, Serialize)]
pub struct MessageDto {
    pub text: String, // Truncated to 500 chars
}

#[derive(Debug, Serialize)]
pub struct TokenUsageDto {
    pub input: u64,
    pub output: u64,
}

impl TurnStepsResponse {
    pub fn from_turn(
        session_id: String,
        turn_index: usize,
        turn: AgentTurn,
        include_reasoning: bool,
        include_tools: bool,
        include_message: bool,
    ) -> Self {
        let steps = turn
            .steps
            .iter()
            .enumerate()
            .map(|(idx, step)| {
                StepDetailDto::from_step(
                    idx,
                    step,
                    include_reasoning,
                    include_tools,
                    include_message,
                )
            })
            .collect();

        let response = Self {
            session_id,
            turn_index,
            turn_id: turn.id.to_string(),
            timestamp: turn.timestamp,
            steps,
            _meta: ResponseMeta::from_bytes(0),
        };

        response.with_metadata(include_reasoning, include_tools, include_message)
    }

    pub fn with_metadata(
        mut self,
        include_reasoning: bool,
        include_tools: bool,
        include_message: bool,
    ) -> Self {
        if let Ok(json) = serde_json::to_string(&self) {
            let bytes = json.len();
            let mut truncated_fields = Vec::new();

            if include_reasoning {
                truncated_fields.push("reasoning.text".to_string());
            }
            if include_tools {
                truncated_fields.push("tools[].result_summary".to_string());
            }
            if include_message {
                truncated_fields.push("message.text".to_string());
            }

            let mut meta = ResponseMeta::with_pagination(
                bytes,
                None, // Single page for now
                self.steps.len(),
                Some(self.steps.len()),
            )
            .with_content_level(crate::mcp::dto::common::ContentLevel::Steps);

            if !truncated_fields.is_empty() {
                meta = meta.with_truncation(truncated_fields, 500);
            }

            self._meta = meta;
        }
        self
    }
}

impl StepDetailDto {
    pub fn from_step(
        step_index: usize,
        step: &AgentStep,
        include_reasoning: bool,
        include_tools: bool,
        include_message: bool,
    ) -> Self {
        let status = match step.status {
            StepStatus::Done => StepStatusDto::Done,
            StepStatus::InProgress => StepStatusDto::InProgress,
            StepStatus::Failed => StepStatusDto::Failed,
        };

        let reasoning = if include_reasoning {
            step.reasoning.as_ref().map(|r| ReasoningDto {
                text: truncate(&r.content.text, 500),
            })
        } else {
            None
        };

        let tools = if include_tools {
            step.tools
                .iter()
                .map(|t| {
                    let input_summary = format!("{:?}", t.call.content); // Simple debug representation
                    let result_summary = t
                        .result
                        .as_ref()
                        .map(|r| truncate(&r.content.output, 200))
                        .unwrap_or_else(|| "(no result)".to_string());

                    ToolExecutionDto {
                        name: t.call.content.name().to_string(),
                        input_summary,
                        result_summary,
                        is_error: t.is_error,
                    }
                })
                .collect()
        } else {
            Vec::new()
        };

        let message = if include_message {
            step.message.as_ref().map(|m| MessageDto {
                text: truncate(&m.content.text, 500),
            })
        } else {
            None
        };

        let tokens = step.usage.as_ref().map(|u| TokenUsageDto {
            input: u.input_tokens() as u64,
            output: u.output_tokens() as u64,
        });

        Self {
            step_index,
            status,
            reasoning,
            tools,
            message,
            tokens,
        }
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s.chars().take(max_len - 3).collect::<String>())
    } else {
        s.to_string()
    }
}
