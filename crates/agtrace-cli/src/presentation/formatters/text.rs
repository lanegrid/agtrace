/// Truncate text to max_len characters, adding "..." if truncated
pub fn truncate(text: &str, max_len: usize) -> String {
    if text.chars().count() <= max_len {
        text.to_string()
    } else {
        let chars: Vec<char> = text.chars().take(max_len - 3).collect();
        format!("{}...", chars.iter().collect::<String>())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_short_text() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_long_text() {
        assert_eq!(truncate("hello world!", 8), "hello...");
    }

    #[test]
    fn test_normalize_and_clean() {
        let input = "<command-name>/clear</command-name>  hello\n\nworld  ";
        assert_eq!(normalize_and_clean(input, 20), "hello world");
    }
}
