use agtrace_types::{AgentOp, AgentToolArgs, ToolCallPayload, ToolKind, ToolOrigin};
use serde_json::Value;

use crate::claude::tools::{
    ClaudeBashArgs, ClaudeEditArgs, ClaudeGlobArgs, ClaudeGrepArgs, ClaudeReadArgs,
    ClaudeTodoWriteArgs, ClaudeWriteArgs,
};

/// Normalize Claude-specific tool calls
///
/// Handles Claude Code provider-specific tool names and maps them to domain variants.
/// Uses provider-specific Args structs for proper schema parsing and conversion.
pub(crate) fn normalize_claude_tool_call(
    tool_name: String,
    arguments: serde_json::Value,
    provider_call_id: Option<String>,
) -> ToolCallPayload {
    if let Some(args) = agent_tool_args(&tool_name, &arguments) {
        return ToolCallPayload::Agent {
            name: tool_name,
            arguments: args,
            provider_call_id,
        };
    }

    // Handle Claude Code-specific tools with provider-specific types
    match tool_name.as_str() {
        "Read" => {
            // Parse as Claude-specific Args, then convert to domain model
            if let Ok(claude_args) = serde_json::from_value::<ClaudeReadArgs>(arguments.clone()) {
                return ToolCallPayload::FileRead {
                    name: tool_name,
                    arguments: claude_args.to_file_read_args(),
                    provider_call_id,
                };
            }
        }
        "Glob" => {
            // Parse as Claude-specific Args, then convert to domain model
            if let Ok(claude_args) = serde_json::from_value::<ClaudeGlobArgs>(arguments.clone()) {
                return ToolCallPayload::FileRead {
                    name: tool_name,
                    arguments: claude_args.to_file_read_args(),
                    provider_call_id,
                };
            }
        }
        "Edit" => {
            // Parse as Claude-specific Args, then convert to domain model
            if let Ok(claude_args) = serde_json::from_value::<ClaudeEditArgs>(arguments.clone()) {
                return ToolCallPayload::FileEdit {
                    name: tool_name,
                    arguments: claude_args.to_file_edit_args(),
                    provider_call_id,
                };
            }
        }
        "Write" => {
            // Parse as Claude-specific Args, then convert to domain model
            if let Ok(claude_args) = serde_json::from_value::<ClaudeWriteArgs>(arguments.clone()) {
                return ToolCallPayload::FileWrite {
                    name: tool_name,
                    arguments: claude_args.to_file_write_args(),
                    provider_call_id,
                };
            }
        }
        "Bash" => {
            // Parse as Claude-specific Args (with timeout, sandbox flags), then convert
            if let Ok(claude_args) = serde_json::from_value::<ClaudeBashArgs>(arguments.clone()) {
                return ToolCallPayload::Execute {
                    name: tool_name,
                    arguments: claude_args.to_execute_args(),
                    provider_call_id,
                };
            }
        }
        "KillShell" | "BashOutput" => {
            // KillShell/BashOutput → Execute (use domain model directly for simpler tools)
            if let Ok(args) = serde_json::from_value(arguments.clone()) {
                return ToolCallPayload::Execute {
                    name: tool_name,
                    arguments: args,
                    provider_call_id,
                };
            }
        }
        "Grep" => {
            // Parse as Claude-specific Args (with many grep-specific options), then convert
            if let Ok(claude_args) = serde_json::from_value::<ClaudeGrepArgs>(arguments.clone()) {
                return ToolCallPayload::Search {
                    name: tool_name,
                    arguments: claude_args.to_search_args(),
                    provider_call_id,
                };
            }
        }
        "WebSearch" | "WebFetch" => {
            // WebSearch/WebFetch → Search (use domain model directly)
            if let Ok(args) = serde_json::from_value(arguments.clone()) {
                return ToolCallPayload::Search {
                    name: tool_name,
                    arguments: args,
                    provider_call_id,
                };
            }
        }
        "TodoWrite" => {
            // Validate as Claude-specific Args, then keep as Generic
            // (no unified Plan variant exists yet in domain model)
            if serde_json::from_value::<ClaudeTodoWriteArgs>(arguments.clone()).is_ok() {
                return ToolCallPayload::Generic {
                    name: tool_name,
                    arguments,
                    provider_call_id,
                };
            }
        }
        _ if tool_name.starts_with("mcp__") => {
            // MCP tools - parse server and tool names using Claude-specific convention
            let (server, tool) = super::tool_mapping::parse_mcp_name(&tool_name)
                .map(|(s, t)| (Some(s), Some(t)))
                .unwrap_or((None, None));

            if let Ok(mut args) =
                serde_json::from_value::<agtrace_types::McpArgs>(arguments.clone())
            {
                args.server = server;
                args.tool = tool;

                return ToolCallPayload::Mcp {
                    name: tool_name,
                    arguments: args,
                    provider_call_id,
                };
            }
        }
        _ => {
            // Unknown Claude tool, fall through to Generic
        }
    }

    // Fallback to generic
    ToolCallPayload::Generic {
        name: tool_name,
        arguments,
        provider_call_id,
    }
}

/// Maximum length (bytes) of prompt / message previews kept in agent tool args.
const PREVIEW_MAX_BYTES: usize = 4096;

/// Normalized arguments of Claude agent-management tools
/// (`Agent`, `SendMessage`, `TaskStop`, `TaskOutput`, `ListAgents`, `SubagentHandback`).
pub(crate) fn agent_tool_args(tool_name: &str, input: &Value) -> Option<AgentToolArgs> {
    let s = |key: &str| {
        input
            .get(key)
            .and_then(Value::as_str)
            .filter(|v| !v.is_empty())
            .map(str::to_string)
    };
    let preview = |key: &str| s(key).map(|v| super::tags::truncate_bytes(&v, PREVIEW_MAX_BYTES));
    let op = match tool_name {
        "Agent" => AgentOp::Spawn,
        "SendMessage" => AgentOp::Send,
        "TaskStop" => AgentOp::Stop,
        "TaskOutput" => AgentOp::Output,
        "ListAgents" => AgentOp::List,
        "SubagentHandback" => AgentOp::Handback,
        _ => return None,
    };
    let mut args = AgentToolArgs::new(op);
    match op {
        AgentOp::Spawn => {
            args.name = s("name");
            args.agent_type = s("subagent_type");
            args.model = s("model");
            args.summary = s("description");
            args.message_preview = preview("prompt");
            args.fork = args.agent_type.as_deref().map(|t| t == "fork");
        }
        AgentOp::Send => {
            args.target = s("to").or_else(|| s("recipient"));
            args.summary = s("summary");
            args.message_preview = preview("message").or_else(|| preview("content"));
        }
        AgentOp::Stop | AgentOp::Output => {
            args.target = s("task_id").or_else(|| s("shell_id"));
        }
        AgentOp::Handback => {
            args.message_preview = preview("message");
        }
        _ => {}
    }
    Some(args)
}

/// Claude tool mapper implementation
pub struct ClaudeToolMapper;

impl crate::traits::ToolMapper for ClaudeToolMapper {
    fn classify(&self, tool_name: &str) -> (ToolOrigin, ToolKind) {
        super::tool_mapping::classify_tool(tool_name)
            .unwrap_or_else(|| crate::tool_analyzer::classify_common(tool_name))
    }

    fn normalize_call(&self, name: &str, args: Value, call_id: Option<String>) -> ToolCallPayload {
        normalize_claude_tool_call(name.to_string(), args, call_id)
    }

    fn summarize(&self, kind: ToolKind, args: &Value) -> String {
        crate::tool_analyzer::extract_common_summary(kind, args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_read() {
        let payload = normalize_claude_tool_call(
            "Read".to_string(),
            serde_json::json!({"file_path": "src/main.rs"}),
            Some("call_123".to_string()),
        );

        match payload {
            ToolCallPayload::FileRead {
                name,
                arguments,
                provider_call_id,
            } => {
                assert_eq!(name, "Read");
                assert_eq!(arguments.file_path, Some("src/main.rs".to_string()));
                assert_eq!(provider_call_id, Some("call_123".to_string()));
            }
            _ => panic!("Expected FileRead variant"),
        }
    }

    #[test]
    fn test_normalize_write() {
        let payload = normalize_claude_tool_call(
            "Write".to_string(),
            serde_json::json!({"file_path": "test.txt", "content": "hello"}),
            Some("call_456".to_string()),
        );

        match payload {
            ToolCallPayload::FileWrite {
                name,
                arguments,
                provider_call_id,
            } => {
                assert_eq!(name, "Write");
                assert_eq!(arguments.file_path, "test.txt");
                assert_eq!(arguments.content, "hello");
                assert_eq!(provider_call_id, Some("call_456".to_string()));
            }
            _ => panic!("Expected FileWrite variant"),
        }
    }

    #[test]
    fn test_normalize_bash() {
        let payload = normalize_claude_tool_call(
            "Bash".to_string(),
            serde_json::json!({"command": "ls -la"}),
            Some("call_789".to_string()),
        );

        match payload {
            ToolCallPayload::Execute {
                name,
                arguments,
                provider_call_id,
            } => {
                assert_eq!(name, "Bash");
                assert_eq!(arguments.command, Some("ls -la".to_string()));
                assert_eq!(provider_call_id, Some("call_789".to_string()));
            }
            _ => panic!("Expected Execute variant"),
        }
    }

    #[test]
    fn test_normalize_agent_tools() {
        let spawn = normalize_claude_tool_call(
            "Agent".to_string(),
            serde_json::json!({"description":"Fork: check docs","prompt":"Check docs.","subagent_type":"fork","model":"opus"}),
            Some("toolu_synthetic_1".to_string()),
        );
        let ToolCallPayload::Agent {
            arguments,
            provider_call_id,
            ..
        } = &spawn
        else {
            panic!("expected Agent, got {spawn:?}");
        };
        assert_eq!(arguments.op, AgentOp::Spawn);
        assert_eq!(arguments.fork, Some(true));
        assert_eq!(arguments.model.as_deref(), Some("opus"));
        assert_eq!(arguments.summary.as_deref(), Some("Fork: check docs"));
        assert_eq!(arguments.message_preview.as_deref(), Some("Check docs."));
        assert_eq!(provider_call_id.as_deref(), Some("toolu_synthetic_1"));
        assert_eq!(spawn.kind(), ToolKind::Agent);

        let send = normalize_claude_tool_call(
            "SendMessage".to_string(),
            serde_json::json!({"to":"audit-A","summary":"s","message":"m","type":"message","recipient":"audit-A","content":"m"}),
            None,
        );
        let ToolCallPayload::Agent { arguments, .. } = send else {
            panic!()
        };
        assert_eq!(arguments.op, AgentOp::Send);
        assert_eq!(arguments.target.as_deref(), Some("audit-A"));
        assert_eq!(arguments.message_preview.as_deref(), Some("m"));

        let stop = normalize_claude_tool_call(
            "TaskStop".to_string(),
            serde_json::json!({"task_id":"audit-A"}),
            None,
        );
        let ToolCallPayload::Agent { arguments, .. } = stop else {
            panic!()
        };
        assert_eq!(arguments.op, AgentOp::Stop);
        assert_eq!(arguments.target.as_deref(), Some("audit-A"));

        for (name, op) in [
            ("TaskOutput", AgentOp::Output),
            ("ListAgents", AgentOp::List),
            ("SubagentHandback", AgentOp::Handback),
        ] {
            let p = normalize_claude_tool_call(name.to_string(), serde_json::json!({}), None);
            let ToolCallPayload::Agent { arguments, .. } = p else {
                panic!("{name}")
            };
            assert_eq!(arguments.op, op);
        }
    }

    #[test]
    fn test_normalize_mcp_tool_parses_server_and_tool() {
        let payload = normalize_claude_tool_call(
            "mcp__o3__o3-search".to_string(),
            serde_json::json!({"input": "test query"}),
            Some("call_mcp".to_string()),
        );

        match payload {
            ToolCallPayload::Mcp {
                name,
                arguments,
                provider_call_id,
            } => {
                assert_eq!(name, "mcp__o3__o3-search");
                assert_eq!(arguments.server, Some("o3".to_string()));
                assert_eq!(arguments.tool, Some("o3-search".to_string()));
                assert_eq!(
                    arguments.inner.get("input").and_then(|v| v.as_str()),
                    Some("test query")
                );
                assert_eq!(provider_call_id, Some("call_mcp".to_string()));
            }
            _ => panic!("Expected Mcp variant"),
        }
    }

    #[test]
    fn test_normalize_mcp_tool_handles_malformed_name() {
        let payload = normalize_claude_tool_call(
            "mcp__invalid".to_string(),
            serde_json::json!({"query": "test"}),
            None,
        );

        match payload {
            ToolCallPayload::Mcp {
                name,
                arguments,
                provider_call_id,
            } => {
                assert_eq!(name, "mcp__invalid");
                // Malformed MCP name should result in None for both server and tool
                assert_eq!(arguments.server, None);
                assert_eq!(arguments.tool, None);
                assert_eq!(provider_call_id, None);
            }
            _ => panic!("Expected Mcp variant"),
        }
    }
}
