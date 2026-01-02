use agtrace_sdk::types::{AgentSession, AgentStep, AgentTurn, SessionStats, StepStatus};
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::mcp::dto::{
    common::{ResponseMeta, truncate_string},
    tool_summary::ToolSummarizer,
};

const MAX_SNIPPET_LEN: usize = 200;

/// Session turns response for get_session_turns tool
/// Target size: 10-30 KB per page (paginated)
#[derive(Debug, Serialize)]
pub struct SessionTurnsResponse {
    pub session_id: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub stats: SessionStats,
    pub turns: Vec<TurnDetailDto>,
    pub _meta: ResponseMeta,
}

#[derive(Debug, Serialize)]
pub struct TurnDetailDto {
    /// Global turn index (always 0-based position in full session, not page-relative)
    pub turn_index: usize,
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub user_message: String,
    pub steps: Vec<StepSummaryDto>,
    pub outcome: TurnOutcome,
    /// Structured key actions for agent-friendly parsing
    pub key_actions: Vec<KeyAction>,
}

#[derive(Debug, Serialize)]
pub struct StepSummaryDto {
    pub step_index: usize,
    pub summary: String,
    pub tool_calls: usize,
    pub failed_tools: usize,
    pub tokens: Option<TokenSummary>,
}

#[derive(Debug, Serialize)]
pub struct TokenSummary {
    pub input: u64,
    pub output: u64,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TurnOutcome {
    Success,
    Partial,
    Failed,
}

/// Structured key action for agent-friendly parsing
#[derive(Debug, Serialize)]
pub struct KeyAction {
    /// Type of action
    #[serde(rename = "type")]
    pub kind: KeyActionKind,
    /// Details specific to the action type
    pub details: serde_json::Value,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KeyActionKind {
    /// Tools were used
    ToolUsage,
    /// Tool executions failed
    Failure,
    /// Steps were incomplete
    Incomplete,
}

impl SessionTurnsResponse {
    #[allow(dead_code)]
    pub fn from_session(session: AgentSession, include_reasoning: bool) -> Self {
        let turns = session
            .turns
            .into_iter()
            .enumerate()
            .map(|(idx, turn)| TurnDetailDto::from_turn(idx, turn, include_reasoning))
            .collect();

        Self {
            session_id: session.session_id.to_string(),
            start_time: session.start_time,
            end_time: session.end_time,
            stats: session.stats,
            turns,
            _meta: ResponseMeta::from_bytes(0), // Placeholder
        }
    }

    /// Create paginated response with metadata
    pub fn from_session_paginated(
        session: AgentSession,
        offset: usize,
        limit: usize,
        next_cursor: Option<String>,
    ) -> Self {
        let total_turns = session.turns.len();
        // IMPORTANT: turn_index must be global index (not page-relative)
        // So we enumerate first, then skip
        let turns: Vec<_> = session
            .turns
            .into_iter()
            .enumerate()
            .skip(offset)
            .take(limit)
            .map(|(global_idx, turn)| TurnDetailDto::from_turn(global_idx, turn, false))
            .collect();

        let response = Self {
            session_id: session.session_id.to_string(),
            start_time: session.start_time,
            end_time: session.end_time,
            stats: session.stats,
            turns,
            _meta: ResponseMeta::from_bytes(0),
        };

        response.with_metadata(next_cursor, total_turns, offset)
    }

    pub fn with_metadata(
        mut self,
        next_cursor: Option<String>,
        total_turns: usize,
        _offset: usize,
    ) -> Self {
        if let Ok(json) = serde_json::to_string(&self) {
            let bytes = json.len();
            let truncated_fields = vec!["user_message".to_string()];
            self._meta = ResponseMeta::with_pagination(
                bytes,
                next_cursor,
                self.turns.len(),
                Some(total_turns),
            )
            .with_content_level(crate::mcp::dto::common::ContentLevel::Turns)
            .with_truncation(truncated_fields, MAX_SNIPPET_LEN);
        }
        self
    }
}

impl TurnDetailDto {
    pub fn from_turn(turn_index: usize, turn: AgentTurn, include_reasoning: bool) -> Self {
        let steps: Vec<StepSummaryDto> = turn
            .steps
            .iter()
            .enumerate()
            .map(|(idx, step)| StepSummaryDto::from_step(idx, step, include_reasoning))
            .collect();

        let outcome = Self::determine_outcome(&turn.steps);
        let key_actions = Self::extract_key_actions(&turn.steps);

        Self {
            turn_index,
            id: turn.id.to_string(),
            timestamp: turn.timestamp,
            user_message: truncate_string(&turn.user.content.text, MAX_SNIPPET_LEN),
            steps,
            outcome,
            key_actions,
        }
    }

    fn determine_outcome(steps: &[AgentStep]) -> TurnOutcome {
        let total_tools: usize = steps.iter().map(|s| s.tools.len()).sum();
        let failed_tools: usize = steps
            .iter()
            .flat_map(|s| &s.tools)
            .filter(|t| t.is_error)
            .count();

        if total_tools == 0 {
            return TurnOutcome::Success;
        }

        if failed_tools == 0 {
            TurnOutcome::Success
        } else if failed_tools < total_tools {
            TurnOutcome::Partial
        } else {
            TurnOutcome::Failed
        }
    }

    fn extract_key_actions(steps: &[AgentStep]) -> Vec<KeyAction> {
        let mut actions = Vec::new();

        // Collect unique tool types used
        let mut tool_types: Vec<String> = Vec::new();
        for step in steps {
            for tool_exec in &step.tools {
                let tool_name = tool_exec.call.content.name().to_string();
                if !tool_types.contains(&tool_name) {
                    tool_types.push(tool_name);
                }
            }
        }

        if !tool_types.is_empty() {
            actions.push(KeyAction {
                kind: KeyActionKind::ToolUsage,
                details: serde_json::json!({
                    "tools": tool_types,
                    "count": tool_types.len(),
                }),
            });
        }

        // Count successful vs failed tools
        let total_tools: usize = steps.iter().map(|s| s.tools.len()).sum();
        let failed_tools: usize = steps
            .iter()
            .flat_map(|s| &s.tools)
            .filter(|t| t.is_error)
            .count();

        if failed_tools > 0 {
            // Collect names of failed tools
            let failed_tool_names: Vec<String> = steps
                .iter()
                .flat_map(|s| &s.tools)
                .filter(|t| t.is_error)
                .map(|t| t.call.content.name().to_string())
                .collect();

            actions.push(KeyAction {
                kind: KeyActionKind::Failure,
                details: serde_json::json!({
                    "failed_count": failed_tools,
                    "total_count": total_tools,
                    "failed_tools": failed_tool_names,
                }),
            });
        }

        // Check for incomplete steps
        let incomplete = steps
            .iter()
            .filter(|s| matches!(s.status, StepStatus::InProgress))
            .count();
        if incomplete > 0 {
            actions.push(KeyAction {
                kind: KeyActionKind::Incomplete,
                details: serde_json::json!({
                    "count": incomplete,
                }),
            });
        }

        actions
    }
}

impl StepSummaryDto {
    pub fn from_step(step_index: usize, step: &AgentStep, include_reasoning: bool) -> Self {
        let summary = Self::generate_summary(step, include_reasoning);
        let tool_calls = step.tools.len();
        let failed_tools = step.tools.iter().filter(|t| t.is_error).count();
        let tokens = step.usage.as_ref().map(|u| TokenSummary {
            input: u.input_tokens() as u64,
            output: u.output_tokens() as u64,
        });

        Self {
            step_index,
            summary,
            tool_calls,
            failed_tools,
            tokens,
        }
    }

    fn generate_summary(step: &AgentStep, include_reasoning: bool) -> String {
        let mut parts = Vec::new();

        // Reasoning summary (if requested and present)
        if include_reasoning && let Some(reasoning) = &step.reasoning {
            let text = truncate_string(&reasoning.content.text, 100);
            parts.push(format!("Thinking: {}", text));
        }

        // Tool executions summary
        if !step.tools.is_empty() {
            let tool_summaries: Vec<String> = step
                .tools
                .iter()
                .map(|t| {
                    ToolSummarizer::summarize_execution(
                        &t.call.content,
                        t.result.as_ref().map(|r| &r.content),
                        t.is_error,
                    )
                })
                .collect();

            parts.push(tool_summaries.join(", "));
        }

        // Message summary
        if let Some(message) = &step.message {
            let text = truncate_string(&message.content.text, 100);
            parts.push(format!("Response: {}", text));
        }

        if parts.is_empty() {
            "No significant activity".to_string()
        } else {
            parts.join(" | ")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_determine_outcome() {
        // Empty steps should be success
        assert_eq!(TurnDetailDto::determine_outcome(&[]), TurnOutcome::Success);
    }

    #[test]
    fn test_extract_key_actions() {
        // Empty steps should return empty actions
        let actions = TurnDetailDto::extract_key_actions(&[]);
        assert!(actions.is_empty());
    }
}
