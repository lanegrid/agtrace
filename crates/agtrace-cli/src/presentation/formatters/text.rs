pub fn truncate(text: &str, max_len: usize) -> String {
    let char_count = text.chars().count();

    if char_count <= max_len {
        text.to_string()
    } else if max_len <= 3 {
        // For very small max_len, just take first chars without "..."
        text.chars().take(max_len).collect()
    } else {
        let truncated: String = text.chars().take(max_len - 3).collect();
        format!("{}...", truncated)
    }
}

/// Smart truncation for file paths - shows end of path (filename) when truncating
pub fn truncate_path(path: &str, max_len: usize) -> String {
    let char_count = path.chars().count();

    if char_count <= max_len {
        return path.to_string();
    }

    if max_len <= 3 {
        return path.chars().take(max_len).collect();
    }

    // Try to show the filename and some parent context
    // Format: ".../parent/filename" (with slash after ...)
    let ellipsis = "...";

    // Take last chars, accounting for ellipsis
    let available = max_len - ellipsis.len();

    let suffix: String = path
        .chars()
        .rev()
        .take(available)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    // If suffix starts with '/', great! Otherwise, skip to first '/'
    if suffix.starts_with('/') {
        format!("{}{}", ellipsis, suffix)
    } else if let Some(slash_pos) = suffix.find('/') {
        // Skip partial component before first '/', add one char for the '/' we're adding after ...
        let trimmed = &suffix[slash_pos..]; // includes the '/'
        format!("{}{}", ellipsis, trimmed)
    } else {
        // No slash found, just use as-is
        format!("{}{}", ellipsis, suffix)
    }
}

/// Format empty or whitespace-only strings for display
pub fn format_empty(text: &str) -> String {
    if text.trim().is_empty() {
        "(empty)".to_string()
    } else {
        text.to_string()
    }
}

/// Smart truncation for multiline text - shows first N lines with line length limits
pub fn truncate_multiline(text: &str, max_lines: usize, max_line_length: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();

    if lines.is_empty() {
        return text.to_string();
    }

    let total_lines = lines.len();
    let lines_to_show = max_lines.min(total_lines);
    let truncated_lines: Vec<String> = lines
        .iter()
        .take(lines_to_show)
        .map(|line| {
            if line.len() > max_line_length {
                truncate(line, max_line_length)
            } else {
                line.to_string()
            }
        })
        .collect();

    let result = truncated_lines.join("\n");

    // Add indicator if there are more lines
    if total_lines > lines_to_show {
        format!("{}\n...", result)
    } else {
        result
    }
}

/// Normalize whitespace, strip known noise, and truncate
pub fn normalize_and_clean(text: &str, max_chars: usize) -> String {
    let normalized = text
        .replace(['\n', '\r'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    let cleaned = normalized
        .trim_start_matches("<command-name>/clear</command-name>")
        .trim_start_matches("<command-message>clear</command-message>")
        .trim()
        .to_string();

    truncate(&cleaned, max_chars)
}

/// Replace home directory with tilde for compact display
pub fn shorten_home_path(path: &str) -> String {
    if let Ok(home) = std::env::var("HOME")
        && path.starts_with(&home)
    {
        return path.replacen(&home, "~", 1);
    }
    path.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_short_text() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_long_text() {
        assert_eq!(truncate("hello world", 8), "hello...");
        assert_eq!(truncate("abcdefghij", 7), "abcd...");
    }

    #[test]
    fn test_truncate_path_short() {
        assert_eq!(truncate_path("/usr/bin", 20), "/usr/bin");
    }

    #[test]
    fn test_truncate_path_long() {
        let path = "/foo-bar-hoge-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/crates/cli/src/main.rs";
        let result = truncate_path(path, 40);

        // Should end with filename and parent dirs, with clean slash after ...
        assert!(result.starts_with(".../"));
        assert!(result.ends_with("main.rs"));
        assert!(result.len() <= 40);
    }

    #[test]
    fn test_truncate_path_shows_filename() {
        let path = "/very/long/path/to/some/important/file.txt";
        let result = truncate_path(path, 25);

        assert!(result.starts_with(".../"));
        assert!(result.contains("file.txt"));
        assert!(result.len() <= 25);
    }

    #[test]
    fn test_format_empty_strings() {
        assert_eq!(format_empty(""), "(empty)");
        assert_eq!(format_empty("   "), "(empty)");
        assert_eq!(format_empty("\n\t"), "(empty)");
        assert_eq!(format_empty("not empty"), "not empty");
    }

    #[test]
    fn test_normalize_and_clean() {
        assert_eq!(normalize_and_clean("hello\nworld", 20), "hello world");
        assert_eq!(
            normalize_and_clean("  multiple   spaces  ", 30),
            "multiple spaces"
        );
    }

    #[test]
    fn test_truncate_multiline_short() {
        let text = "line1\nline2\nline3";
        assert_eq!(truncate_multiline(text, 5, 100), "line1\nline2\nline3");
    }

    #[test]
    fn test_truncate_multiline_lines_limit() {
        let text = "line1\nline2\nline3\nline4\nline5";
        let result = truncate_multiline(text, 3, 100);
        assert_eq!(result, "line1\nline2\nline3\n...");
    }

    #[test]
    fn test_truncate_multiline_line_length() {
        let text = "this is a very long line that should be truncated\nshort";
        let result = truncate_multiline(text, 5, 20);
        assert!(result.contains("this is a very lo..."));
        assert!(result.contains("short"));
    }

    #[test]
    fn test_truncate_multiline_both_limits() {
        let text = "line1 is very long indeed\nline2 is also quite lengthy\nline3\nline4\nline5";
        let result = truncate_multiline(text, 2, 20);
        assert_eq!(result.lines().count(), 3); // 2 lines + "..."
        assert!(result.contains("..."));
    }

    #[test]
    fn test_shorten_home_path() {
        // Test with actual HOME env var
        if let Ok(home) = std::env::var("HOME") {
            let full_path = format!("{}/projects/test", home);
            let shortened = shorten_home_path(&full_path);
            assert_eq!(shortened, "~/projects/test");

            // Test path not starting with HOME
            let other_path = "/usr/local/bin";
            assert_eq!(shorten_home_path(other_path), other_path);
        }
    }
}
