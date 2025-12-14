use crate::session_loader::{LoadOptions, SessionLoader};
use agtrace_engine::build_spans;
use agtrace_index::Database;
use agtrace_types::resolve_effective_project_hash;
use anyhow::Result;

pub fn handle(db: &Database, project_hash: Option<String>, all_projects: bool) -> Result<()> {
    let (effective_hash_string, _all_projects) =
        resolve_effective_project_hash(project_hash.as_deref(), all_projects)?;
    let effective_project_hash = effective_hash_string.as_deref();

    // Use a larger pool and balance
    let raw_sessions = db.list_sessions(effective_project_hash, 500)?;

    if raw_sessions.is_empty() {
        println!("No sessions found.");
        return Ok(());
    }

    println!(
        "# Corpus Overview (Sample: {} sessions)",
        raw_sessions.len()
    );

    let loader = SessionLoader::new(db);
    let options = LoadOptions::default();

    let mut total_tool_calls = 0;
    let mut total_failures = 0;
    let mut max_duration = 0;

    // Simple one-pass metrics
    for session in &raw_sessions {
        if let Ok(events) = loader.load_events_v2(&session.id, &options) {
            // Just quick aggregations for the header
            let spans = build_spans(&events);
            for span in spans {
                total_tool_calls += span.stats.tool_calls;
                total_failures += span.stats.tool_failures;
                if let Some(ms) = span.stats.e2e_ms {
                    if ms > max_duration {
                        max_duration = ms;
                    }
                }
            }
        }
    }

    println!("Total Tool Calls: {}", total_tool_calls);
    println!("Total Failures: {}", total_failures);
    println!("Max Duration: {:.1}s", max_duration as f64 / 1000.0);
    println!("\nUse `agtrace pack --template diagnose` to see actionable problem sessions.");

    Ok(())
}
