use agtrace_types::ToolKind;
use serde_json::Value;

/// Classify Codex tool by semantic kind
pub fn classify_tool(tool_name: &str) -> Option<ToolKind> {
    match tool_name {
        "apply_patch" => Some(ToolKind::Write),
        "read_mcp_resource" => Some(ToolKind::Read),
        "shell" | "shell_command" => Some(ToolKind::Execute),
        "update_plan" => Some(ToolKind::Plan),
        _ => None,
    }
}

/// Extract summary from Codex tool arguments
pub fn extract_summary(tool_name: &str, _kind: ToolKind, arguments: &Value) -> Option<String> {
    match tool_name {
        // apply_patch: extract filename from raw patch
        "apply_patch" => extract_patch_filename(arguments),

        // shell: join array command
        "shell" => extract_shell_command(arguments),

        // update_plan: extract explanation
        "update_plan" => arguments
            .get("explanation")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),

        // Default: None (use common logic)
        _ => None,
    }
}

fn extract_patch_filename(args: &Value) -> Option<String> {
    let raw = args.get("raw")?.as_str()?;
    // Try to extract "*** Update File: path/to/file.rs"
    let start = raw.find("Update File: ")?;
    let rest = &raw[start + 13..];
    let end = rest.find('\n').unwrap_or(rest.len());
    Some(rest[..end].trim().to_string())
}

fn extract_shell_command(args: &Value) -> Option<String> {
    let cmd_array = args.get("command")?.as_array()?;
    let cmd_str = cmd_array
        .iter()
        .filter_map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    Some(cmd_str)
}
