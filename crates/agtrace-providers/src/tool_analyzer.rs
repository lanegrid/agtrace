use agtrace_types::{ToolKind, ToolOrigin};
use serde_json::Value;

/// Common classification logic used as fallback when provider doesn't recognize a tool
pub fn classify_common(tool_name: &str) -> (ToolOrigin, ToolKind) {
    let origin = determine_origin(tool_name);
    let kind = classify_kind(tool_name);
    (origin, kind)
}

/// Common summary extraction logic used as fallback
pub fn extract_common_summary(kind: ToolKind, arguments: &Value) -> String {
    let candidate = match kind {
        ToolKind::Execute => pick_first(arguments, &["command", "cmd"]),
        ToolKind::Read | ToolKind::Write => pick_first(arguments, &["file_path", "path", "uri"]),
        ToolKind::Search => pick_first(arguments, &["pattern", "query", "input"]),
        ToolKind::Plan => pick_first(arguments, &["description", "explanation"]),
        ToolKind::Ask => pick_first(arguments, &["prompt", "question"]),
        _ => None,
    };

    if let Some(val) = candidate {
        format_value(val)
    } else {
        "(args...)".to_string()
    }
}

// --- Internal helpers ---

fn classify_kind(tool_name: &str) -> ToolKind {
    let lower = tool_name.to_lowercase();
    // Check more specific patterns first
    if lower.contains("todo") || lower.contains("plan") {
        ToolKind::Plan
    } else if lower.contains("search") || lower.contains("grep") || lower.contains("find") {
        ToolKind::Search
    } else if lower.contains("ask") || lower.contains("prompt") || lower.contains("question") {
        ToolKind::Ask
    } else if lower.contains("read") || lower.contains("get") || lower.contains("fetch") {
        ToolKind::Read
    } else if lower.contains("write")
        || lower.contains("edit")
        || lower.contains("update")
        || lower.contains("patch")
    {
        ToolKind::Write
    } else if lower.contains("shell")
        || lower.contains("bash")
        || lower.contains("exec")
        || lower.contains("run")
        || lower.contains("command")
    {
        ToolKind::Execute
    } else {
        ToolKind::Other
    }
}

/// Determine tool origin based on naming convention
///
/// MCP protocol tools are identified by the `mcp__` prefix. This indicates
/// the tool is invoked via MCP protocol, not that it operates on MCP resources.
/// For example:
/// - `mcp__sqlite__query` → MCP (external tool via MCP protocol)
/// - `read_mcp_resource` → System (provider-native tool that reads MCP resources)
fn determine_origin(tool_name: &str) -> ToolOrigin {
    if tool_name.starts_with("mcp__") {
        ToolOrigin::Mcp
    } else {
        ToolOrigin::System
    }
}

fn pick_first<'a>(args: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    for key in keys {
        if let Some(val) = args.get(key)
            && !val.is_null()
        {
            return Some(val);
        }
    }
    None
}

fn format_value(val: &Value) -> String {
    match val {
        Value::String(s) => truncate(s, 80),
        Value::Array(arr) => {
            let joined = arr
                .iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            truncate(&joined, 80)
        }
        _ => truncate(&val.to_string(), 80),
    }
}

pub fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_classify_common_heuristics() {
        assert_eq!(
            classify_common("search_database"),
            (ToolOrigin::System, ToolKind::Search)
        );
        assert_eq!(
            classify_common("read_config"),
            (ToolOrigin::System, ToolKind::Read)
        );
        assert_eq!(
            classify_common("write_log"),
            (ToolOrigin::System, ToolKind::Write)
        );
        assert_eq!(
            classify_common("run_command"),
            (ToolOrigin::System, ToolKind::Execute)
        );
        assert_eq!(
            classify_common("update_plan"),
            (ToolOrigin::System, ToolKind::Plan)
        );
    }

    #[test]
    fn test_classify_common_mcp() {
        assert_eq!(
            classify_common("mcp__custom_tool"),
            (ToolOrigin::Mcp, ToolKind::Other)
        );
        assert_eq!(
            classify_common("mcp__search_docs"),
            (ToolOrigin::Mcp, ToolKind::Search)
        );
    }

    #[test]
    fn test_extract_common_summary() {
        let args = json!({"command": "cargo test", "description": "Run tests"});
        assert_eq!(
            extract_common_summary(ToolKind::Execute, &args),
            "cargo test"
        );

        let args = json!({"file_path": "/path/to/file.rs"});
        assert_eq!(
            extract_common_summary(ToolKind::Read, &args),
            "/path/to/file.rs"
        );

        let args = json!({"pattern": "fn main"});
        assert_eq!(extract_common_summary(ToolKind::Search, &args), "fn main");

        let args = json!({"unknown_key": "value"});
        assert_eq!(extract_common_summary(ToolKind::Other, &args), "(args...)");
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("short", 80), "short");
        assert_eq!(
            truncate(&"a".repeat(100), 80),
            format!("{}...", "a".repeat(77))
        );
    }
}
