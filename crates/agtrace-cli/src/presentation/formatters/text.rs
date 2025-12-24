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
