use crate::db::{Database, SessionSummary};
use crate::utils::resolve_effective_project_hash;
use anyhow::Result;

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

/// Truncate string for display, respecting UTF-8 character boundaries
fn truncate_for_display(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars - 3).collect();
        format!("{}...", truncated)
    }
}

fn print_sessions_table(sessions: &[SessionSummary]) {
    println!(
        "{:<25} {:<12} {:<12} {:<25} {:<30}",
        "TIME", "PROVIDER", "ID (short)", "PROJECT", "SNIPPET"
    );
    println!("{}", "-".repeat(120));

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

        let time_str = session
            .start_ts
            .as_deref()
            .unwrap_or("unknown");

        let time_display = if time_str.len() > 19 {
            &time_str[..19]
        } else {
            time_str
        };

        let snippet = session
            .snippet
            .as_deref()
            .unwrap_or("");
        let snippet_display = truncate_for_display(snippet, 30);

        println!(
            "{:<25} {:<12} {:<12} {:<25} {:<30}",
            time_display,
            session.provider,
            id_short,
            project_short,
            snippet_display
        );
    }
}
