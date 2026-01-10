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

/// Parse MCP tool name into server and tool components
///
/// MCP tools follow the pattern: mcp__{server}__{tool}
/// Example: "mcp__o3__search" -> ("o3", "search")
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

/// Check if a tool is MCP based on display name
///
/// Gemini uses a different naming convention for MCP tools:
/// - Tool name: "o3-search" (no mcp__ prefix)
/// - Display name: "o3-search (o3 MCP Server)"
///
/// This function extracts the server name from the display name pattern.
pub fn is_mcp_tool(display_name: Option<&str>) -> bool {
    display_name
        .map(|name| name.contains("MCP Server"))
        .unwrap_or(false)
}

/// Extract MCP server name from display name
///
/// Examples:
/// - "o3-search (o3 MCP Server)" -> Some("o3")
/// - "brave-search (brave MCP Server)" -> Some("brave")
/// - "regular-tool" -> None
pub fn extract_mcp_server_name(display_name: Option<&str>) -> Option<String> {
    let name = display_name?;

    // Look for pattern: "xxx (server MCP Server)"
    let start = name.find('(')?;
    let end = name.find(" MCP Server)")?;

    if start < end {
        let server = name[start + 1..end].trim();
        Some(server.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_mcp_tool() {
        assert!(is_mcp_tool(Some("o3-search (o3 MCP Server)")));
        assert!(is_mcp_tool(Some("brave-search (brave MCP Server)")));
        assert!(!is_mcp_tool(Some("Google Web Search")));
        assert!(!is_mcp_tool(Some("read_file")));
        assert!(!is_mcp_tool(None));
    }

    #[test]
    fn test_extract_mcp_server_name() {
        assert_eq!(
            extract_mcp_server_name(Some("o3-search (o3 MCP Server)")),
            Some("o3".to_string())
        );
        assert_eq!(
            extract_mcp_server_name(Some("brave-search (brave MCP Server)")),
            Some("brave".to_string())
        );
        assert_eq!(
            extract_mcp_server_name(Some("sqlite-query (sqlite MCP Server)")),
            Some("sqlite".to_string())
        );
        assert_eq!(extract_mcp_server_name(Some("Google Web Search")), None);
        assert_eq!(extract_mcp_server_name(Some("read_file")), None);
        assert_eq!(extract_mcp_server_name(None), None);
    }
}
