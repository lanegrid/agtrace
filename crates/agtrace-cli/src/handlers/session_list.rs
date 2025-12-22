use crate::args::OutputFormat;
use crate::presentation::renderers::TraceView;
use crate::presentation::view_models::SessionListEntryViewModel;
use agtrace_runtime::{AgTrace, SessionFilter};
use anyhow::Result;
use std::path::Path;

#[allow(clippy::too_many_arguments)]
pub fn handle(
    workspace: &AgTrace,
    project_root: Option<&Path>,
    all_projects: bool,
    project_hash: Option<String>,
    limit: usize,
    format: OutputFormat,
    source: Option<String>,
    since: Option<String>,
    until: Option<String>,
    no_auto_refresh: bool,
    view: &dyn TraceView,
) -> Result<()> {
    // Note: index::handle call removed - Read-Through Indexing handles this automatically

    // Build filter
    let mut filter = SessionFilter::new().limit(limit);

    if let Some(hash) = project_hash {
        filter = filter.project(hash);
    }

    if all_projects {
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
