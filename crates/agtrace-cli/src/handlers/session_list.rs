use crate::context::ExecutionContext;
use crate::presentation::renderers::TraceView;
use crate::presentation::view_models::SessionListEntryViewModel;
use crate::types::OutputFormat;
use agtrace_index::Database;
use anyhow::Result;
use std::path::Path;

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
    no_auto_refresh: bool,
    data_dir: &Path,
    project_root: Option<String>,
    view: &dyn TraceView,
) -> Result<()> {
    // Auto-refresh index before listing (unless disabled)
    if !no_auto_refresh {
        let ctx =
            ExecutionContext::new(data_dir.to_path_buf(), project_root.clone(), all_projects)?;

        // Run incremental scan quietly (verbose=false)
        if let Err(e) = crate::handlers::index::handle(&ctx, "all".to_string(), false, false, view)
        {
            // Don't fail the list command if refresh fails - just warn
            view.render_warning(&format!("Warning: auto-refresh failed: {}", e))?;
        }
    }

    let service = agtrace_runtime::SessionService::new(db);
    let sessions = service.list_sessions(agtrace_runtime::ListSessionsRequest {
        project_hash,
        limit,
        all_projects,
        source,
        since,
        until,
    })?;

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
