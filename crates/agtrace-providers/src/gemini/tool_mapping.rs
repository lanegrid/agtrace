use agtrace_types::{ToolKind, ToolOrigin};
use serde_json::Value;

/// Classify Gemini tool by origin and semantic kind
pub fn classify_tool(tool_name: &str) -> Option<(ToolOrigin, ToolKind)> {
    let (origin, kind) = match tool_name {
        "google_web_search" => (ToolOrigin::System, ToolKind::Search),
        "read_file" => (ToolOrigin::System, ToolKind::Read),
        "replace" | "write_file" => (ToolOrigin::System, ToolKind::Write),
        "run_shell_command" => (ToolOrigin::System, ToolKind::Execute),
        "write_todos" => (ToolOrigin::System, ToolKind::Plan),
        _ if tool_name.starts_with("mcp__") => (ToolOrigin::Mcp, ToolKind::Other),
        _ => return None,
    };

    Some((origin, kind))
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
