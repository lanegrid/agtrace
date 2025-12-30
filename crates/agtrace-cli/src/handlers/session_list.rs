use crate::args::{OutputFormat, ViewModeArgs};
use agtrace_runtime::{AgTrace, SessionFilter};
use anyhow::Result;
use std::path::Path;

#[allow(clippy::too_many_arguments)]
pub fn handle(
    workspace: &AgTrace,
    _project_root: Option<&Path>,
    all_projects: bool,
    project_hash: Option<agtrace_types::ProjectHash>,
    limit: usize,
    format: OutputFormat,
    provider: Option<String>,
    since: Option<String>,
    until: Option<String>,
    no_auto_refresh: bool,
    view_mode: &ViewModeArgs,
) -> Result<()> {
    use crate::presentation::presenters;
    use crate::presentation::{ConsoleRenderer, Renderer};

    // Build filter
    let mut filter = SessionFilter::new().limit(limit);

    let project_filter_summary = project_hash.as_ref().map(|h| h.to_string());

    if let Some(hash) = project_hash {
        filter = filter.project(hash);
    }

    if all_projects {
        filter = filter.all_projects();
    }

    if let Some(ref src) = provider {
        filter = filter.provider(src.clone());
    }

    if let Some(ref since_str) = since {
        filter = filter.since(since_str.clone());
    }

    if let Some(ref until_str) = until {
        filter = filter.until(until_str.clone());
    }

    // Get sessions (with optional auto-refresh)
    let sessions = if no_auto_refresh {
        workspace.sessions().list_without_refresh(filter)?
    } else {
        workspace.sessions().list(filter)?
    };

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
        provider,
        time_range,
        limit,
    );

    let presentation_format = crate::presentation::OutputFormat::from(format);
    let resolved_view_mode = view_mode.resolve();
    let renderer = ConsoleRenderer::new(presentation_format, resolved_view_mode);
    renderer.render(view_model)?;

    Ok(())
}
