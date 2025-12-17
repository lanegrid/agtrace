use crate::context::ExecutionContext;
use crate::session_loader::{LoadOptions, SessionLoader};
use crate::views::pack::{print_compact, print_diagnose, print_tools};
use agtrace_engine::{analyze_and_select_sessions, assemble_session_from_events, SessionDigest};
use agtrace_index::SessionSummary;
use agtrace_types::resolve_effective_project_hash;
use anyhow::Result;
use std::collections::HashMap;

pub fn handle(
    ctx: &ExecutionContext,
    template: &str,
    limit: usize,
    project_hash: Option<String>,
) -> Result<()> {
    let db = ctx.db()?;
    let all_projects = ctx.all_projects;
    let (effective_hash_string, _all_projects) =
        resolve_effective_project_hash(project_hash.as_deref(), all_projects)?;
    let effective_project_hash = effective_hash_string.as_deref();

    // 1. Collect: Balance sessions by provider to avoid bias
    // Fetch enough raw sessions to ensure we can balance them
    let raw_sessions = db.list_sessions(effective_project_hash, 1000)?;
    let balanced_sessions = balance_sessions_by_provider(&raw_sessions, 200);

    println!(
        "# Packing Report (pool: {} sessions from {} raw candidates)\n",
        balanced_sessions.len(),
        raw_sessions.len()
    );

    // 2. Summarize: Calculate metrics for all candidates
    let mut digests = Vec::new();
    let loader = SessionLoader::new(db);
    let options = LoadOptions::default();

    for (i, session) in balanced_sessions.iter().enumerate() {
        if let Ok(events) = loader.load_events_v2(&session.id, &options) {
            if let Some(agent_session) = assemble_session_from_events(&events) {
                // Newer sessions get a small boost in scoring
                let recency_boost = (balanced_sessions.len() - i) as u32;
                let digest = SessionDigest::new(
                    &session.id,
                    &session.provider,
                    agent_session,
                    recency_boost,
                );
                digests.push(digest);
            }
        }
    }

    // 3. Select: Apply lenses with deduplication
    let selections = analyze_and_select_sessions(digests, limit);

    // Output based on template
    match template {
        "compact" => print_compact(&selections),
        "diagnose" => print_diagnose(&selections),
        "tools" => print_tools(&selections),
        _ => print_compact(&selections),
    }

    Ok(())
}

// --- Core Logic Implementation ---

fn balance_sessions_by_provider(
    sessions: &[SessionSummary],
    target_per_provider: usize,
) -> Vec<SessionSummary> {
    let mut by_provider: HashMap<String, Vec<SessionSummary>> = HashMap::new();
    for session in sessions {
        by_provider
            .entry(session.provider.clone())
            .or_default()
            .push(session.clone());
    }

    let mut balanced = Vec::new();
    for (_, mut list) in by_provider {
        // Keep most recent ones from each provider
        if list.len() > target_per_provider {
            list.truncate(target_per_provider);
        }
        balanced.extend(list);
    }

    // Sort by timestamp desc to keep overall chronological order slightly
    balanced.sort_by(|a, b| b.start_ts.cmp(&a.start_ts));
    balanced
}
