use crate::context::ExecutionContext;
use crate::presentation::renderers::TraceView;
use crate::presentation::view_models::SessionListEntryViewModel;
use crate::types::OutputFormat;
use agtrace_runtime::SessionFilter;
use anyhow::Result;

#[allow(clippy::too_many_arguments)]
pub fn handle(
    ctx: &ExecutionContext,
    project_hash: Option<String>,
    limit: usize,
    format: OutputFormat,
    source: Option<String>,
    since: Option<String>,
    until: Option<String>,
    no_auto_refresh: bool,
    view: &dyn TraceView,
) -> Result<()> {
    let workspace = ctx.workspace()?;

    // Auto-refresh index before listing (unless disabled)
    if !no_auto_refresh {
        // Run incremental scan quietly (verbose=false)
        if let Err(e) = crate::handlers::index::handle(ctx, "all".to_string(), false, false, view)
        {
            // Don't fail the list command if refresh fails - just warn
            view.render_warning(&format!("Warning: auto-refresh failed: {}", e))?;
        }
    }

    // Build filter
    let mut filter = SessionFilter::new().limit(limit);

    if let Some(hash) = project_hash {
        filter = filter.project(hash);
    }

    if ctx.all_projects {
        filter = filter.all_projects();
    }

    if let Some(src) = source {
        filter = filter.source(src);
    }

    if let Some(since_str) = since {
        filter = filter.since(since_str);
    }

    if let Some(until_str) = until {
        filter = filter.until(until_str);
    }

    // Get sessions
    let sessions = workspace.sessions().list(filter)?;

    // Convert to ViewModels
    let session_vms: Vec<SessionListEntryViewModel> = sessions
        .into_iter()
        .map(|s| SessionListEntryViewModel {
            id: s.id,
            provider: s.provider,
            project_hash: s.project_hash,
            start_ts: s.start_ts,
            snippet: s.snippet,
        })
        .collect();

    view.render_session_list(&session_vms, format)?;

    Ok(())
}
