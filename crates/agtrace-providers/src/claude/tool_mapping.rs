use crate::tool_spec::ToolSpec;
use agtrace_types::{ToolKind, ToolOrigin};
use serde_json::Value;

/// Registry of Claude Code tools
const CLAUDE_TOOLS: &[ToolSpec] = &[
    // Ask tools
    ToolSpec::new("AskUserQuestion", ToolOrigin::System, ToolKind::Ask),
    // Execute tools
    ToolSpec::new("Bash", ToolOrigin::System, ToolKind::Execute),
    ToolSpec::new("KillShell", ToolOrigin::System, ToolKind::Execute),
    ToolSpec::new("BashOutput", ToolOrigin::System, ToolKind::Execute),
    ToolSpec::new("Skill", ToolOrigin::System, ToolKind::Execute),
    ToolSpec::new("SlashCommand", ToolOrigin::System, ToolKind::Execute),
    // Write tools
    ToolSpec::new("Edit", ToolOrigin::System, ToolKind::Write),
    ToolSpec::new("Write", ToolOrigin::System, ToolKind::Write),
    ToolSpec::new("NotebookEdit", ToolOrigin::System, ToolKind::Write),
    // Read tools
    ToolSpec::new("Read", ToolOrigin::System, ToolKind::Read),
    // Search tools
    ToolSpec::new("Glob", ToolOrigin::System, ToolKind::Search),
    ToolSpec::new("Grep", ToolOrigin::System, ToolKind::Search),
    ToolSpec::new("WebFetch", ToolOrigin::System, ToolKind::Search),
    ToolSpec::new("WebSearch", ToolOrigin::System, ToolKind::Search),
    // Plan tools
    ToolSpec::new("Task", ToolOrigin::System, ToolKind::Plan),
    ToolSpec::new("TaskCreate", ToolOrigin::System, ToolKind::Plan),
    ToolSpec::new("TaskUpdate", ToolOrigin::System, ToolKind::Plan),
    ToolSpec::new("TaskGet", ToolOrigin::System, ToolKind::Plan),
    ToolSpec::new("TaskList", ToolOrigin::System, ToolKind::Plan),
    ToolSpec::new("TaskStop", ToolOrigin::System, ToolKind::Plan),
    ToolSpec::new("TaskOutput", ToolOrigin::System, ToolKind::Plan),
    ToolSpec::new("TodoWrite", ToolOrigin::System, ToolKind::Plan),
    ToolSpec::new("EnterPlanMode", ToolOrigin::System, ToolKind::Plan),
    ToolSpec::new("ExitPlanMode", ToolOrigin::System, ToolKind::Plan),
];

/// Classify Claude Code tool by origin and semantic kind
pub fn classify_tool(tool_name: &str) -> Option<(ToolOrigin, ToolKind)> {
    // Check registry first
    if let Some(spec) = CLAUDE_TOOLS.iter().find(|t| t.name == tool_name) {
        return Some((spec.origin, spec.kind));
    }

    // Handle MCP protocol tools (invoked via MCP, not just accessing MCP resources)
    // These tools are prefixed with "mcp__" and are external integrations
    if tool_name.starts_with("mcp__") {
        return Some((ToolOrigin::Mcp, ToolKind::Other));
    }

    None
}

/// Extract summary from Claude Code tool arguments
pub fn extract_summary(tool_name: &str, _kind: ToolKind, arguments: &Value) -> Option<String> {
    match tool_name {
        // AskUserQuestion: extract header from questions array
        "AskUserQuestion" => arguments
            .get("questions")
            .and_then(|q| q.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("header"))
            .and_then(|h| h.as_str())
            .map(|s| s.to_string()),

        // TodoWrite: summarize todo list
        "TodoWrite" => extract_todo_summary(arguments),

        // BashOutput: show bash_id
        "BashOutput" => arguments
            .get("bash_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),

        // Default: None (use common logic)
        _ => None,
    }
}

fn extract_todo_summary(args: &Value) -> Option<String> {
    let todos = args.get("todos")?.as_array()?;
    let count = todos.len();
    let first = todos.first()?;
    let text = first.get("content")?.as_str()?;

    if count > 1 {
        Some(format!("{} (+{} more)", text, count - 1))
    } else {
        Some(text.to_string())
    }
}

// ==========================================
// MCP Tool Name Parsing (Claude Code)
// ==========================================

/// Parse MCP tool name from full name (e.g., "mcp__o3__o3-search" -> ("o3", "o3-search"))
///
/// Claude Code MCP tools follow the naming convention: `mcp__{server}__{tool}`
pub fn parse_mcp_name(full_name: &str) -> Option<(String, String)> {
    if !full_name.starts_with("mcp__") {
        return None;
    }

    let rest = &full_name[5..]; // Remove "mcp__"
    let parts: Vec<&str> = rest.splitn(2, "__").collect();

    if parts.len() == 2 {
        Some((parts[0].to_string(), parts[1].to_string()))
    } else {
        None
    }
}

/// Get server name from full MCP tool name
pub fn mcp_server_name(full_name: &str) -> Option<String> {
    parse_mcp_name(full_name).map(|(server, _)| server)
}

/// Get tool name from full MCP tool name
pub fn mcp_tool_name(full_name: &str) -> Option<String> {
    parse_mcp_name(full_name).map(|(_, tool)| tool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mcp_name() {
        assert_eq!(
            parse_mcp_name("mcp__o3__o3-search"),
            Some(("o3".to_string(), "o3-search".to_string()))
        );

        assert_eq!(
            parse_mcp_name("mcp__sqlite__query"),
            Some(("sqlite".to_string(), "query".to_string()))
        );

        assert_eq!(parse_mcp_name("not_mcp_tool"), None);
        assert_eq!(parse_mcp_name("mcp__only_server"), None);
    }

    #[test]
    fn test_mcp_server_and_tool_name() {
        assert_eq!(
            mcp_server_name("mcp__o3__o3-search"),
            Some("o3".to_string())
        );
        assert_eq!(
            mcp_tool_name("mcp__o3__o3-search"),
            Some("o3-search".to_string())
        );

        assert_eq!(mcp_server_name("not_mcp_tool"), None);
        assert_eq!(mcp_tool_name("not_mcp_tool"), None);
    }
}
