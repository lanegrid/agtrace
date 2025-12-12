use agtrace_index::{Database, SessionSummary};
use agtrace_types::resolve_effective_project_hash;
use anyhow::Result;
use chrono::{DateTime, Utc};
use comfy_table::{presets::UTF8_FULL, Cell, Color, ContentArrangement, Table};

pub fn handle(
    db: &Database,
    project_hash: Option<String>,
    limit: usize,
    all_projects: bool,
    format: &str,
) -> Result<()> {
    let (effective_hash_string, _all_projects) =
        resolve_effective_project_hash(project_hash.as_deref(), all_projects)?;
    let effective_project_hash = effective_hash_string.as_deref();

    let sessions = db.list_sessions(effective_project_hash, limit)?;

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&sessions)?);
    } else {
        print_sessions_table(&sessions);
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
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("TIME").fg(Color::White),
            Cell::new("PROVIDER").fg(Color::White),
            Cell::new("ID").fg(Color::White),
            Cell::new("PROJECT").fg(Color::White),
            Cell::new("SNIPPET").fg(Color::White),
        ]);

    for session in sessions {
        let id_short = if session.id.len() > 8 {
            &session.id[..8]
        } else {
            &session.id
        };

        let project_short = if session.project_hash.len() > 12 {
            format!("{}...", &session.project_hash[..9])
        } else {
            session.project_hash.clone()
        };

        let time_str = session.start_ts.as_deref().unwrap_or("unknown");
        let time_display = format_relative_time(time_str);

        let snippet = session.snippet.as_deref().unwrap_or("");
        let snippet_display = truncate_for_display(snippet, 70);

        // Color code cells
        let time_cell = Cell::new(time_display).fg(Color::Cyan);

        let provider_color = match session.provider.as_str() {
            "claude" => Color::Blue,
            "codex" => Color::Green,
            "gemini" => Color::Red,
            _ => Color::White,
        };
        let provider_cell = Cell::new(&session.provider).fg(provider_color);

        let id_cell = Cell::new(id_short).fg(Color::Yellow);
        let project_cell = Cell::new(project_short).fg(Color::DarkGrey);

        let snippet_cell = if snippet_display.is_empty() {
            Cell::new("[empty]").fg(Color::DarkGrey)
        } else {
            Cell::new(snippet_display).fg(Color::White)
        };

        table.add_row(vec![
            time_cell,
            provider_cell,
            id_cell,
            project_cell,
            snippet_cell,
        ]);
    }

    println!("{table}");
}
