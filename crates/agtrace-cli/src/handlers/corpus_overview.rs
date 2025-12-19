use crate::context::ExecutionContext;
use crate::session_loader::{LoadOptions, SessionLoader};
use crate::ui::models::CorpusStats;
use crate::ui::TraceView;
use agtrace_engine::assemble_session;
use agtrace_types::resolve_effective_project_hash;
use anyhow::Result;

pub fn handle(
    ctx: &ExecutionContext,
    project_hash: Option<String>,
    view: &dyn TraceView,
) -> Result<()> {
    let db = ctx.db()?;
    let all_projects = ctx.all_projects;
    let (effective_hash_string, _all_projects) =
        resolve_effective_project_hash(project_hash.as_deref(), all_projects)?;
    let effective_project_hash = effective_hash_string.as_deref();

    // Use a larger pool and balance
    let raw_sessions = db.list_sessions(effective_project_hash, 500)?;

    let loader = SessionLoader::new(db);
    let options = LoadOptions::default();

    let mut total_tool_calls = 0;
    let mut total_failures = 0;
    let mut max_duration = 0i64;

    // Simple one-pass metrics
    for session in &raw_sessions {
        if let Ok(events) = loader.load_events(&session.id, &options) {
            // Just quick aggregations for the header
            if let Some(agent_session) = assemble_session(&events) {
                for turn in &agent_session.turns {
                    for step in &turn.steps {
                        total_tool_calls += step.tools.len();
                        for tool_exec in &step.tools {
                            if tool_exec.is_error {
                                total_failures += 1;
                            }
                        }
                    }
                    if turn.stats.duration_ms > max_duration {
                        max_duration = turn.stats.duration_ms;
                    }
                }
            }
        }
    }

    view.render_corpus_overview(&CorpusStats {
        sample_size: raw_sessions.len(),
        total_tool_calls,
        total_failures,
        max_duration_ms: max_duration,
    })?;

    Ok(())
}
