use crate::types::OutputFormat;
use agtrace_index::{Database, SessionSummary};
use agtrace_types::resolve_effective_project_hash;
use anyhow::Result;
use chrono::{DateTime, Utc};

#[allow(clippy::too_many_arguments)]
pub fn handle(
    db: &Database,
    project_hash: Option<String>,
    limit: usize,
    all_projects: bool,
    format: OutputFormat,
    source: Option<String>,
    since: Option<String>,
    until: Option<String>,
) -> Result<()> {
    let (effective_hash_string, _all_projects) =
        resolve_effective_project_hash(project_hash.as_deref(), all_projects)?;
    let effective_project_hash = effective_hash_string.as_deref();

    // Fetch more sessions to allow filtering
    let fetch_limit = limit * 3;
    let mut sessions = db.list_sessions(effective_project_hash, fetch_limit)?;

    // Filter by source (provider)
    if let Some(src) = source {
        sessions.retain(|s| s.provider == src);
    }

    // Filter by since (start_ts >= since)
    if let Some(since_str) = since {
        if let Ok(since_dt) = DateTime::parse_from_rfc3339(&since_str) {
            sessions.retain(|s| {
                if let Some(ts) = &s.start_ts {
                    if let Ok(dt) = DateTime::parse_from_rfc3339(ts) {
                        return dt >= since_dt;
                    }
                }
                false
            });
        }
    }

    // Filter by until (start_ts <= until)
    if let Some(until_str) = until {
        if let Ok(until_dt) = DateTime::parse_from_rfc3339(&until_str) {
            sessions.retain(|s| {
                if let Some(ts) = &s.start_ts {
                    if let Ok(dt) = DateTime::parse_from_rfc3339(ts) {
                        return dt <= until_dt;
                    }
                }
                false
            });
        }
    }

    // Apply limit after filtering
    sessions.truncate(limit);

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&sessions)?);
        }
        OutputFormat::Plain => {
            print_sessions_table(&sessions);
        }
    }

    Ok(())
}

/// Truncate and normalize string for display
/// - Replaces newlines with spaces
/// - Collapses multiple consecutive whitespace into single space
/// - Respects UTF-8 character boundaries
/// - Removes common prefixes and noise
fn truncate_for_display(s: &str, max_chars: usize) -> String {
    // Replace newlines with spaces and collapse multiple spaces
    let normalized = s
        .replace(['\n', '\r'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    // Remove common noise patterns
    let cleaned = normalized
        .trim_start_matches("<command-name>/clear</command-name>")
        .trim_start_matches("<command-message>clear</command-message>")
        .trim()
        .to_string();

    if cleaned.chars().count() <= max_chars {
        cleaned
    } else {
        let truncated: String = cleaned.chars().take(max_chars - 3).collect();
        format!("{}...", truncated)
    }
}

/// Format timestamp as relative time (e.g., "2 hours ago", "yesterday")
fn format_relative_time(ts: &str) -> String {
    let parsed = match DateTime::parse_from_rfc3339(ts) {
        Ok(dt) => dt.with_timezone(&Utc),
        Err(_) => return ts.to_string(),
    };

    let now = Utc::now();
    let duration = now.signed_duration_since(parsed);

    let seconds = duration.num_seconds();
    let minutes = duration.num_minutes();
    let hours = duration.num_hours();
    let days = duration.num_days();

    if seconds < 60 {
        "just now".to_string()
    } else if minutes < 60 {
        format!("{} min ago", minutes)
    } else if hours < 24 {
        format!("{} hours ago", hours)
    } else if days == 1 {
        "yesterday".to_string()
    } else if days < 7 {
        format!("{} days ago", days)
    } else if days < 30 {
        let weeks = days / 7;
        format!("{} weeks ago", weeks)
    } else if days < 365 {
        let months = days / 30;
        format!("{} months ago", months)
    } else {
        let years = days / 365;
        format!("{} years ago", years)
    }
}

fn print_sessions_table(sessions: &[SessionSummary]) {
    use owo_colors::OwoColorize;

    for session in sessions {
        let id_short = if session.id.len() > 8 {
            &session.id[..8]
        } else {
            &session.id
        };

        let time_str = session.start_ts.as_deref().unwrap_or("unknown");
        let time_display = format_relative_time(time_str);

        let snippet = session.snippet.as_deref().unwrap_or("");
        let snippet_display = truncate_for_display(snippet, 80);

        let provider_display = match session.provider.as_str() {
            "claude_code" => format!("{}", session.provider.blue()),
            "codex" => format!("{}", session.provider.green()),
            "gemini" => format!("{}", session.provider.red()),
            _ => session.provider.clone(),
        };

        let snippet_final = if snippet_display.is_empty() {
            format!("{}", "[empty]".bright_black())
        } else {
            snippet_display
        };

        println!(
            "{} {} {} {}",
            time_display.bright_black(),
            id_short.yellow(),
            provider_display,
            snippet_final
        );
    }
}
