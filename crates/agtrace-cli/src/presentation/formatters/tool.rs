use super::{path, text};
use owo_colors::OwoColorize;

/// Categorize a tool by name, returning an icon and color function
pub fn categorize_tool(name: &str) -> (&'static str, fn(&str) -> String) {
    let lower = name.to_lowercase();

    if lower.contains("read")
        || lower.contains("ls")
        || lower.contains("cat")
        || lower.contains("grep")
        || lower.contains("search")
        || lower.contains("view")
    {
        ("📖", |s: &str| s.cyan().to_string())
    } else if lower.contains("write") || lower.contains("edit") || lower.contains("replace") {
        ("🛠️", |s: &str| s.yellow().to_string())
    } else if lower.contains("run")
        || lower.contains("exec")
        || lower.contains("bash")
        || lower.contains("python")
        || lower.contains("test")
    {
        ("🧪", |s: &str| s.magenta().to_string())
    } else {
        ("🔧", |s: &str| s.white().to_string())
    }
}

/// Format tool call arguments into a summary string
pub fn format_tool_call(
    _name: &str,
    args: &serde_json::Value,
    project_root: Option<&std::path::Path>,
) -> String {
    if let Some(obj) = args.as_object() {
        if let Some(p) = obj.get("path").or_else(|| obj.get("file_path")) {
            if let Some(path_str) = p.as_str() {
                let shortened = path::shorten_path(path_str, project_root);
                return format!("(\"{}\")", text::truncate(&shortened, 60));
            }
        }
        if let Some(command) = obj.get("command") {
            if let Some(cmd_str) = command.as_str() {
                return format!("(\"{}\")", text::truncate(cmd_str, 60));
            }
        }
        if let Some(pattern) = obj.get("pattern") {
            if let Some(pat_str) = pattern.as_str() {
                return format!("(\"{}\")", text::truncate(pat_str, 60));
            }
        }
    }

    let json = args.to_string();
    format!("({})", text::truncate(&json, 40))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_categorize_read_tool() {
        let (icon, _) = categorize_tool("Read");
        assert_eq!(icon, "📖");
    }

    #[test]
    fn test_categorize_write_tool() {
        let (icon, _) = categorize_tool("Write");
        assert_eq!(icon, "🛠️");
    }

    #[test]
    fn test_categorize_exec_tool() {
        let (icon, _) = categorize_tool("Bash");
        assert_eq!(icon, "🧪");
    }

    #[test]
    fn test_categorize_unknown_tool() {
        let (icon, _) = categorize_tool("Unknown");
        assert_eq!(icon, "🔧");
    }
}
