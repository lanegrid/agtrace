use agtrace_types::ToolKind;
use serde_json::Value;

/// Classify Claude Code tool by semantic kind
pub fn classify_tool(tool_name: &str) -> Option<ToolKind> {
    match tool_name {
        "AskUserQuestion" => Some(ToolKind::Ask),
        "Bash" | "KillShell" | "BashOutput" => Some(ToolKind::Execute),
        "Edit" | "Write" | "NotebookEdit" => Some(ToolKind::Write),
        "Read" => Some(ToolKind::Read),
        "Glob" | "Grep" => Some(ToolKind::Search),
        "Task" | "TodoWrite" | "ExitPlanMode" => Some(ToolKind::Plan),
        "Skill" | "SlashCommand" => Some(ToolKind::Execute),
        "WebFetch" | "WebSearch" => Some(ToolKind::Search),
        _ => None,
    }
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
