use crate::tool_spec::ToolSpec;
use agtrace_types::{ToolKind, ToolOrigin};
use serde_json::Value;

/// Registry of Gemini tools
const GEMINI_TOOLS: &[ToolSpec] = &[
    // Search tools
    ToolSpec::new("google_web_search", ToolOrigin::System, ToolKind::Search),
    // Read tools
    ToolSpec::new("read_file", ToolOrigin::System, ToolKind::Read),
    // Write tools
    ToolSpec::new("replace", ToolOrigin::System, ToolKind::Write),
    ToolSpec::new("write_file", ToolOrigin::System, ToolKind::Write),
    // Execute tools
    ToolSpec::new("run_shell_command", ToolOrigin::System, ToolKind::Execute),
    // Plan tools
    ToolSpec::new("write_todos", ToolOrigin::System, ToolKind::Plan),
];

/// Classify Gemini tool by origin and semantic kind
pub fn classify_tool(tool_name: &str) -> Option<(ToolOrigin, ToolKind)> {
    // Check registry first
    if let Some(spec) = GEMINI_TOOLS.iter().find(|t| t.name == tool_name) {
        return Some((spec.origin, spec.kind));
    }

    // Handle MCP protocol tools (invoked via MCP, not just accessing MCP resources)
    // These tools are prefixed with "mcp__" and are external integrations
    if tool_name.starts_with("mcp__") {
        return Some((ToolOrigin::Mcp, ToolKind::Other));
    }

    None
}

/// Extract summary from Gemini tool arguments
pub fn extract_summary(tool_name: &str, _kind: ToolKind, arguments: &Value) -> Option<String> {
    match tool_name {
        // write_todos: summarize todo list
        "write_todos" => extract_todo_summary(arguments),

        // Default: None (use common logic)
        _ => None,
    }
}

fn extract_todo_summary(args: &Value) -> Option<String> {
    let todos = args.get("todos")?.as_array()?;
    let count = todos.len();
    let first = todos.first()?;
    // Gemini uses "description" instead of "content"
    let text = first.get("description")?.as_str()?;

    if count > 1 {
        Some(format!("{} (+{} more)", text, count - 1))
    } else {
        Some(text.to_string())
    }
}
