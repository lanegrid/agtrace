use chrono::{DateTime, Utc};
use std::path::Path;

/// Format project path to show only last 2 components
/// Example: /Users/zawakin/go/src/github.com/lanegrid/lanegrid → lanegrid/lanegrid
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

/// Format date to short format (e.g., "Nov 22")
pub fn format_date_short(dt: &DateTime<Utc>) -> String {
    dt.format("%b %d").to_string()
}

/// Format ID to show only first 8 characters
pub fn format_id_short(id: &str) -> String {
    if id.len() > 8 {
        id[..8].to_string()
    } else {
        id.to_string()
    }
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
