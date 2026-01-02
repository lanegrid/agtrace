use agtrace_sdk::types::{
    AgentStep, AgentTurn, ContextWindowUsage, ToolCallPayload, ToolExecution, TurnStats,
    UserMessage,
};
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::mcp::models::common::{truncate_json_value, truncate_string};

/// Get detailed steps for a specific turn (20-50 KB)
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetTurnStepsArgs {
    /// Session ID obtained from list_sessions or get_session_turns response.
    /// Accepts 8-character prefix (e.g., "fb3cff44") or full UUID.
    /// REQUIRED: Cannot be empty.
    pub session_id: String,

    /// Zero-based turn index (obtained from get_session_turns response).
    /// REQUIRED: Must be valid (0 to turn_count - 1).
    pub turn_index: usize,
}

#[derive(Debug, Serialize)]
pub struct TurnStepsViewModel {
    pub session_id: String,
    pub turn_index: usize,
    pub turn: TurnDetail,
}

impl TurnStepsViewModel {
    pub fn new(session_id: String, turn_index: usize, turn: AgentTurn) -> Self {
        Self {
            session_id,
            turn_index,
            turn: TurnDetail::from(turn),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct TurnDetail {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub user: UserMessage,
    pub steps: Vec<StepDetail>,
    pub stats: TurnStats,
}

impl From<AgentTurn> for TurnDetail {
    fn from(turn: AgentTurn) -> Self {
        Self {
            id: turn.id,
            timestamp: turn.timestamp,
            user: turn.user,
            steps: turn.steps.into_iter().map(StepDetail::from).collect(),
            stats: turn.stats,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct StepDetail {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    pub tools: Vec<ToolExecutionDetail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<ContextWindowUsage>,
    pub is_failed: bool,
    pub status: String,
}

impl From<AgentStep> for StepDetail {
    fn from(step: AgentStep) -> Self {
        Self {
            id: step.id,
            timestamp: step.timestamp,
            reasoning: step
                .reasoning
                .map(|r| truncate_string(&r.content.text, 200)),
            message: step.message.map(|m| m.content.text),
            tools: step
                .tools
                .into_iter()
                .map(ToolExecutionDetail::from)
                .collect(),
            usage: step.usage,
            is_failed: step.is_failed,
            status: format!("{:?}", step.status).to_lowercase(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ToolExecutionDetail {
    pub name: String,
    pub arguments: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<i64>,
    pub is_error: bool,
}

impl From<ToolExecution> for ToolExecutionDetail {
    fn from(tool: ToolExecution) -> Self {
        let name = tool.call.content.name().to_string();
        let arguments = truncate_tool_arguments(&tool.call.content);
        let result = tool
            .result
            .map(|r| truncate_tool_result(&name, &r.content.output));

        Self {
            name,
            arguments,
            result,
            duration_ms: tool.duration_ms,
            is_error: tool.is_error,
        }
    }
}

/// Truncate tool arguments based on tool type, preserving essential information
fn truncate_tool_arguments(payload: &ToolCallPayload) -> Value {
    match payload {
        ToolCallPayload::FileRead {
            name, arguments, ..
        } => {
            serde_json::json!({
                "name": name,
                "file_path": arguments.file_path,
                "path": arguments.path,
                "pattern": arguments.pattern,
            })
        }
        ToolCallPayload::FileEdit {
            name, arguments, ..
        } => {
            serde_json::json!({
                "name": name,
                "file_path": &arguments.file_path,
                "old_string": truncate_string(&arguments.old_string, 100),
                "new_string": truncate_string(&arguments.new_string, 100),
                "replace_all": arguments.replace_all,
            })
        }
        ToolCallPayload::FileWrite {
            name, arguments, ..
        } => {
            serde_json::json!({
                "name": name,
                "file_path": &arguments.file_path,
                "content": truncate_string(&arguments.content, 100),
            })
        }
        ToolCallPayload::Execute {
            name, arguments, ..
        } => {
            serde_json::json!({
                "name": name,
                "command": arguments.command,
                "description": arguments.description,
                "timeout": arguments.timeout,
            })
        }
        ToolCallPayload::Search {
            name, arguments, ..
        } => {
            serde_json::json!({
                "name": name,
                "pattern": arguments.pattern,
                "query": arguments.query,
                "path": arguments.path,
            })
        }
        ToolCallPayload::Mcp {
            name, arguments, ..
        } => {
            serde_json::json!({
                "name": name,
                "server": arguments.server,
                "tool": arguments.tool,
                "inner": truncate_json_value(&arguments.inner, 100),
            })
        }
        ToolCallPayload::Generic {
            name, arguments, ..
        } => {
            serde_json::json!({
                "name": name,
                "arguments": truncate_json_value(arguments, 100),
            })
        }
    }
}

/// Truncate tool result based on tool type
fn truncate_tool_result(tool_name: &str, output: &str) -> String {
    match tool_name {
        // Bash commands might have long output - keep more context
        "Bash" => truncate_string(output, 1000),
        // MCP tools might have structured output - keep more context
        name if name.starts_with("mcp__") => truncate_string(output, 1000),
        // File operations usually have short output (error messages, success)
        "Read" | "Write" | "Edit" => truncate_string(output, 500),
        // Search results can be long
        "Grep" | "Glob" | "WebSearch" => truncate_string(output, 500),
        // Default
        _ => truncate_string(output, 500),
    }
}
