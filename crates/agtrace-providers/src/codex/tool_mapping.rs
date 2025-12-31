use crate::tool_spec::ToolSpec;
use agtrace_types::{ToolKind, ToolOrigin};
use serde_json::Value;

/// Registry of Codex tools
///
/// # Note on ToolOrigin classification
/// `read_mcp_resource` is classified as System, not Mcp, because it is a
/// provider-native tool built into Codex. The tool happens to read MCP resources,
/// but the invocation itself is NOT via MCP protocol. Only tools invoked via
/// MCP protocol (typically prefixed with `mcp__`) should use ToolOrigin::Mcp.
const CODEX_TOOLS: &[ToolSpec] = &[
    // Write tools
    ToolSpec::new("apply_patch", ToolOrigin::System, ToolKind::Write),
    // Read tools
    // NOTE: read_mcp_resource is System because it's a Codex-native tool,
    // even though it reads MCP resources. Origin is about invocation, not target.
    ToolSpec::new("read_mcp_resource", ToolOrigin::System, ToolKind::Read),
    // Execute tools
    ToolSpec::new("shell", ToolOrigin::System, ToolKind::Execute),
    ToolSpec::new("shell_command", ToolOrigin::System, ToolKind::Execute),
    // Plan tools
    ToolSpec::new("update_plan", ToolOrigin::System, ToolKind::Plan),
];

/// Classify Codex tool by origin and semantic kind
pub fn classify_tool(tool_name: &str) -> Option<(ToolOrigin, ToolKind)> {
    // Check registry first
    if let Some(spec) = CODEX_TOOLS.iter().find(|t| t.name == tool_name) {
        return Some((spec.origin, spec.kind));
    }

    // Handle MCP protocol tools (invoked via MCP, not just accessing MCP resources)
    // These tools are prefixed with "mcp__" and are external integrations
    if tool_name.starts_with("mcp__") {
        return Some((ToolOrigin::Mcp, ToolKind::Other));
    }

    None
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

// ==========================================
// MCP Tool Name Parsing (Codex)
// ==========================================

/// Parse MCP tool name from full name (e.g., "mcp__o3__o3-search" -> ("o3", "o3-search"))
///
/// Codex MCP tools follow the naming convention: `mcp__{server}__{tool}`
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
