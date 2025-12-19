use agtrace_types::ToolKind;
use serde_json::Value;

/// Classify Gemini tool by semantic kind
pub fn classify_tool(tool_name: &str) -> Option<ToolKind> {
    match tool_name {
        "google_web_search" => Some(ToolKind::Search),
        "read_file" => Some(ToolKind::Read),
        "replace" | "write_file" => Some(ToolKind::Write),
        "run_shell_command" => Some(ToolKind::Execute),
        "write_todos" => Some(ToolKind::Plan),
        _ => None,
    }
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
