use agtrace_sdk::types::{
    AgentSession, AgentStep, AgentTurn, SessionStats, ToolCallPayload, TurnStats,
};
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::mcp::models::common::truncate_string;

/// Get turn-level summaries with pagination (10-30 KB per page)
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetSessionTurnsArgs {
    /// Session ID obtained from list_sessions response (use the 'id' field).
    /// Accepts 8-character prefix (e.g., "fb3cff44") or full UUID.
    /// REQUIRED: Cannot be empty.
    pub session_id: String,

    /// Pagination cursor from previous response's next_cursor field. Omit for first page.
    #[serde(default)]
    pub cursor: Option<String>,

    /// Maximum number of turns to return per page (default: 10, max: 50)
    #[serde(default)]
    pub limit: Option<usize>,
}

impl GetSessionTurnsArgs {
    pub fn limit(&self) -> usize {
        self.limit.unwrap_or(10).min(50)
    }
}

#[derive(Debug, Serialize)]
pub struct SessionTurnsViewModel {
    pub session_id: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub stats: SessionStats,
    pub turns: Vec<TurnOverview>,
    pub total_turns: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

impl SessionTurnsViewModel {
    pub fn new(
        session: AgentSession,
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
            .map(|(global_idx, turn)| TurnOverview::new(global_idx, turn))
            .collect();

        Self {
            session_id: session.session_id.to_string(),
            start_time: session.start_time,
            end_time: session.end_time,
            stats: session.stats,
            turns,
            total_turns,
            next_cursor,
        }
    }
}

/// A simplified view of a turn, optimized for low token usage
#[derive(Debug, Serialize)]
pub struct TurnOverview {
    pub turn_index: usize,
    pub input_snippet: String,
    pub step_count: usize,
    /// Simplified steps: only showing tool names and status
    pub steps_summary: Vec<String>,
    /// Truncated output from final step (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_snippet: Option<String>,
    pub stats: TurnStats,
}

impl TurnOverview {
    fn new(index: usize, turn: AgentTurn) -> Self {
        // Truncate user input
        let input_snippet = truncate_string(&turn.user.content.text, 150);

        let step_count = turn.steps.len();

        // Heavily simplify steps to just tool names (no arguments or results)
        // Example: ["Read [OK]", "Write [OK]", "Bash [FAILED]"]
        let steps_summary = turn
            .steps
            .iter()
            .take(5) // Limit to first 5 steps
            .map(summarize_step)
            .collect::<Vec<_>>();

        // Add notation if there are more steps
        let mut final_summary = steps_summary;
        if step_count > 5 {
            final_summary.push(format!("... (+{} more steps)", step_count - 5));
        }

        // Extract final message output (if any)
        let output_snippet = turn
            .steps
            .last()
            .and_then(|s| s.message.as_ref())
            .map(|m| truncate_string(&m.content.text, 150));

        Self {
            turn_index: index,
            input_snippet,
            step_count,
            steps_summary: final_summary,
            output_snippet,
            stats: turn.stats,
        }
    }
}

fn summarize_step(step: &AgentStep) -> String {
    // Collect tool summaries with minimal context from this step
    let tool_summaries: Vec<String> = step
        .tools
        .iter()
        .map(|t| {
            let status = if t.is_error { "FAILED" } else { "OK" };
            let context = extract_tool_context(&t.call.content);

            if context.is_empty() {
                format!("{} [{}]", t.call.content.name(), status)
            } else {
                format!("{} {} [{}]", t.call.content.name(), context, status)
            }
        })
        .collect();

    if tool_summaries.is_empty() {
        // No tools - likely just a message response
        if step.message.is_some() {
            "Message".to_string()
        } else if step.reasoning.is_some() {
            "Reasoning".to_string()
        } else {
            "Empty".to_string()
        }
    } else {
        // Join multiple tool calls with comma
        tool_summaries.join(", ")
    }
}

/// Extract minimal context from tool call for overview purposes
fn extract_tool_context(payload: &ToolCallPayload) -> String {
    match payload {
        ToolCallPayload::FileRead { arguments, .. } => {
            // Show basename of file path
            if let Some(path) = arguments.file_path.as_ref().or(arguments.path.as_ref()) {
                std::path::Path::new(path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(path)
                    .to_string()
            } else if let Some(pattern) = &arguments.pattern {
                truncate_string(pattern, 20)
            } else {
                String::new()
            }
        }
        ToolCallPayload::FileEdit { arguments, .. } => {
            // Show basename of file path
            std::path::Path::new(&arguments.file_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&arguments.file_path)
                .to_string()
        }
        ToolCallPayload::FileWrite { arguments, .. } => {
            // Show basename of file path
            std::path::Path::new(&arguments.file_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&arguments.file_path)
                .to_string()
        }
        ToolCallPayload::Execute { arguments, .. } => {
            // Show first part of command
            arguments
                .command
                .as_ref()
                .map(|cmd| {
                    let truncated = truncate_string(cmd, 30);
                    format!("'{}'", truncated)
                })
                .unwrap_or_default()
        }
        ToolCallPayload::Search { arguments, .. } => {
            // Show search pattern
            if let Some(pattern) = &arguments.pattern {
                format!("'{}'", truncate_string(pattern, 20))
            } else if let Some(query) = &arguments.query {
                format!("'{}'", truncate_string(query, 20))
            } else {
                String::new()
            }
        }
        ToolCallPayload::Mcp { arguments, .. } => {
            // Show server/tool combination
            match (&arguments.server, &arguments.tool) {
                (Some(server), Some(tool)) => format!("{}::{}", server, tool),
                (Some(server), None) => server.clone(),
                (None, Some(tool)) => tool.clone(),
                (None, None) => String::new(),
            }
        }
        ToolCallPayload::Generic { .. } => {
            // Generic tools don't have predictable structure
            String::new()
        }
    }
}
