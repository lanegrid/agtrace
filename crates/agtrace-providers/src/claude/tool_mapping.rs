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
    ToolSpec::new("TodoWrite", ToolOrigin::System, ToolKind::Plan),
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
