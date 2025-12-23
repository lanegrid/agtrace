use crate::args::OutputFormat;
use agtrace_runtime::{AgTrace, SessionFilter};
use anyhow::Result;
use std::path::Path;

/// Resolve ViewMode from CLI flags
fn resolve_view_mode(
    quiet: bool,
    compact: bool,
    verbose: bool,
) -> crate::presentation::v2::ViewMode {
    use crate::presentation::v2::ViewMode;

    if quiet {
        ViewMode::Minimal
    } else if compact {
        ViewMode::Compact
    } else if verbose {
        ViewMode::Verbose
    } else {
        ViewMode::default()
    }
}

#[allow(clippy::too_many_arguments)]
pub fn handle_v2(
    workspace: &AgTrace,
    _project_root: Option<&Path>,
    all_projects: bool,
    project_hash: Option<String>,
    limit: usize,
    format: OutputFormat,
    source: Option<String>,
    since: Option<String>,
    until: Option<String>,
    _no_auto_refresh: bool,
    quiet: bool,
    compact: bool,
    verbose: bool,
) -> Result<()> {
    use crate::presentation::v2::presenters;
    use crate::presentation::v2::{ConsoleRenderer, Renderer};

    // Build filter
    let mut filter = SessionFilter::new().limit(limit);

    let project_filter_summary = project_hash.clone();

    if let Some(hash) = project_hash {
        filter = filter.project(hash);
    }

    if all_projects {
        filter = filter.all_projects();
    }

    if let Some(ref src) = source {
        filter = filter.source(src.clone());
    }

    if let Some(ref since_str) = since {
        filter = filter.since(since_str.clone());
    }

    if let Some(ref until_str) = until {
        filter = filter.until(until_str.clone());
    }

    // Get sessions
    let sessions = workspace.sessions().list(filter)?;

    // Build time range summary
    let time_range = match (since.as_ref(), until.as_ref()) {
        (Some(s), Some(u)) => Some(format!("{} to {}", s, u)),
        (Some(s), None) => Some(format!("since {}", s)),
        (None, Some(u)) => Some(format!("until {}", u)),
        (None, None) => None,
    };

    let view_model = presenters::present_session_list(
        sessions,
        project_filter_summary,
        source,
        time_range,
        limit,
    );

    let v2_format = crate::presentation::v2::OutputFormat::from(format);
    let view_mode = resolve_view_mode(quiet, compact, verbose);
    let renderer = ConsoleRenderer::new(v2_format, view_mode);
    renderer.render(view_model)?;

    Ok(())
}
