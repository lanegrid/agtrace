use crate::model::{Event, Execution};
use chrono::{DateTime, Utc};
use std::path::Path;

/// Format path with home directory replaced by ~
/// Example: /Users/user/projects/foo → ~/projects/foo
pub fn format_path(path: &Path) -> String {
    if let Some(home) = std::env::var_os("HOME") {
        let home_path = Path::new(&home);
        if let Ok(relative) = path.strip_prefix(home_path) {
            return format!("~/{}", relative.display());
        }
    }
    path.display().to_string()
}

/// Format path compactly for table display
/// Shortens long paths by abbreviating intermediate components and keeping
/// only the last directory in full.
/// Example: /Users/user/go/src/github.com/org/project → ~/g/s/.../project
pub fn format_path_compact(path: &Path, max_width: usize) -> String {
    let formatted = format_path(path);

    if formatted.len() <= max_width {
        return formatted;
    }

    // Split into components (already with ~ if under home)
    let parts: Vec<&str> = formatted.split('/').collect();
    if parts.len() <= 2 {
        // Nothing to abbreviate, just truncate visibly
        return truncate_with_ellipsis(&formatted, max_width);
    }

    // Determine head (~, /, or first component)
    let (head, start_idx) = if formatted.starts_with("~/") {
        ("~", 1)
    } else if formatted.starts_with('/') {
        ("/", 1)
    } else {
        (parts[0], 1)
    };

    if parts.len() - start_idx <= 1 {
        return truncate_with_ellipsis(&formatted, max_width);
    }

    let last = parts.last().unwrap();
    let mid = &parts[start_idx..parts.len() - 1];

    // Abbreviate intermediate components to their first character
    let mid_short: Vec<String> = mid
        .iter()
        .map(|s| s.chars().next().unwrap_or('?').to_string())
        .collect();

    // First attempt: head + all abbreviated components + last
    let mut candidate = String::new();
    candidate.push_str(head);
    if head != "/" && !head.is_empty() {
        candidate.push('/');
    }
    candidate.push_str(&mid_short.join("/"));
    candidate.push('/');
    candidate.push_str(last);

    if candidate.len() <= max_width {
        return candidate;
    }

    // Second attempt: head + first abbreviated + ... + last
    let mut candidate2 = String::new();
    candidate2.push_str(head);
    if head != "/" && !head.is_empty() {
        candidate2.push('/');
    }
    if !mid_short.is_empty() {
        candidate2.push_str(&mid_short[0]);
        candidate2.push('/');
    }
    candidate2.push_str("...");
    candidate2.push('/');
    candidate2.push_str(last);

    if candidate2.len() <= max_width {
        return candidate2;
    }

    // Fallback: .../last (or truncated last)
    let fallback = format!(".../{}", last);
    if fallback.len() <= max_width {
        fallback
    } else {
        truncate_with_ellipsis(last, max_width)
    }
}

/// Format project path to show only last 2 components
/// Example: /Users/user/go/src/github.com/org/project → org/project
/// DEPRECATED: Use format_path or format_path_compact instead
#[allow(dead_code)]
pub fn format_project_short(path: &Path) -> String {
    path.iter()
        .rev()
        .take(2)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|s| s.to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// Format duration in seconds to human-readable format
/// Examples: 45s, 5m, 2h15m
pub fn format_duration(seconds: u64) -> String {
    if seconds < 60 {
        format!("{}s", seconds)
    } else if seconds < 3600 {
        format!("{}m", seconds / 60)
    } else {
        format!("{}h{}m", seconds / 3600, (seconds % 3600) / 60)
    }
}

/// Format date to short format (e.g., "2025-11-26")
pub fn format_date_short(dt: &DateTime<Utc>) -> String {
    dt.format("%Y-%m-%d").to_string()
}

/// Format number with commas (e.g., 12345 → "12,345")
pub fn format_number(n: u64) -> String {
    let s = n.to_string();
    let chars: Vec<char> = s.chars().collect();
    let mut result = String::new();

    for (i, c) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(*c);
    }

    result
}

/// Extract and format the first user message as a task snippet
/// Removes special XML tags and truncates to a reasonable length
pub fn format_task_snippet(execution: &Execution, max_len: usize) -> String {
    for event in &execution.events {
        if let Event::UserMessage { content, .. } = event {
            // Skip common system/generated messages
            if is_system_message(content) {
                continue;
            }

            let cleaned = clean_task_content(content);
            if !cleaned.is_empty() && cleaned.len() > 3 {
                // Make sure it's not just whitespace or very short
                return truncate_with_ellipsis(&cleaned, max_len);
            }
        }
    }
    String::from("-")
}

/// Check if a message is a system-generated message that should be skipped
fn is_system_message(content: &str) -> bool {
    let trimmed = content.trim();

    // Common system message patterns
    let system_patterns = [
        "Caveat:",
        "<command-name>",
        "<local-command-stdout>",
        "# Lanegrid Context Package",
    ];

    for pattern in &system_patterns {
        if trimmed.starts_with(pattern) {
            return true;
        }
    }

    // If it's very short or only XML tags, it's probably not a real task
    let cleaned = clean_task_content(content);
    cleaned.is_empty() || cleaned.len() < 10
}

/// Clean task content by removing XML-like tags and extra whitespace
fn clean_task_content(content: &str) -> String {
    let mut result = content.to_string();

    // Remove "Caveat:" prefix section (everything from Caveat to the first actual content)
    if let Some(caveat_pos) = result.find("Caveat:") {
        // Find the end of the caveat section (usually ends with a double newline or after a long sentence)
        if let Some(content_start) = result[caveat_pos..].find("\n\n") {
            result = result[caveat_pos + content_start + 2..].to_string();
        } else if let Some(newline) = result[caveat_pos..].find('\n') {
            result = result[caveat_pos + newline + 1..].to_string();
        }
    }

    // Remove common environment/system tags
    let patterns = [
        ("<command-name>", "</command-name>"),
        ("<command-message>", "</command-message>"),
        ("<command-args>", "</command-args>"),
        ("<environment_context>", "</environment_context>"),
        ("<local-command-stdout>", "</local-command-stdout>"),
    ];

    for (start, end) in patterns {
        while let Some(start_pos) = result.find(start) {
            if let Some(end_pos) = result.find(end) {
                result.replace_range(start_pos..=end_pos + end.len() - 1, "");
            } else {
                break;
            }
        }
    }

    // Collapse multiple whitespace and newlines
    let words: Vec<&str> = result.split_whitespace().collect();
    let cleaned = words.join(" ").trim().to_string();

    // If the result starts with "# " (markdown header), that's probably generated context, skip to next real content
    if cleaned.starts_with("# ") {
        // This is likely a "# Lanegrid Context Package" type message, which isn't the real task
        // Try to find the next user message or real content
        if let Some(hash_end) = cleaned.find("**") {
            // Skip past the generated context header
            return cleaned[hash_end..]
                .split_whitespace()
                .skip(5)
                .collect::<Vec<_>>()
                .join(" ");
        }
    }

    cleaned
}

/// Truncate string with ellipsis if it exceeds max_len
fn truncate_with_ellipsis(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}
