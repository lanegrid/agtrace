use crate::context::ExecutionContext;
use crate::presentation::renderers::TraceView;
use crate::presentation::view_models::CorpusStats;
use agtrace_types::resolve_effective_project_hash;
use anyhow::Result;

pub fn handle(
    ctx: &ExecutionContext,
    project_hash: Option<String>,
    view: &dyn TraceView,
) -> Result<()> {
    let workspace = ctx.workspace()?;
    let all_projects = ctx.all_projects;
    let (effective_hash_string, _all_projects) =
        resolve_effective_project_hash(project_hash.as_deref(), all_projects)?;
    let effective_project_hash = effective_hash_string.as_deref();

    let stats = workspace.insights().corpus_stats(effective_project_hash, 500)?;

    view.render_corpus_overview(&CorpusStats {
        sample_size: stats.sample_size,
        total_tool_calls: stats.total_tool_calls,
        total_failures: stats.total_failures,
        max_duration_ms: stats.max_duration_ms,
    })?;

    Ok(())
}
