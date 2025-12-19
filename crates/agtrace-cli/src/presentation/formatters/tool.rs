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
