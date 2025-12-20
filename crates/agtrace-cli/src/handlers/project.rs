use crate::presentation::renderers::TraceView;
use crate::presentation::view_models::ProjectSummary;
use agtrace_runtime::AgTrace;
use agtrace_types::discover_project_root;
use anyhow::Result;

pub fn handle(
    workspace: &AgTrace,
    project_root: Option<String>,
    view: &dyn TraceView,
) -> Result<()> {
    let project_root_path = discover_project_root(project_root.as_deref())?;
    let project_hash = agtrace_types::project_hash_from_root(&project_root_path.to_string_lossy());

    let projects = workspace.projects().list()?;

    let summaries: Vec<ProjectSummary> = projects
        .into_iter()
        .map(|p| ProjectSummary {
            hash: p.hash,
            root_path: p.root_path,
            session_count: p.session_count,
            last_scanned: p.last_scanned,
        })
        .collect();

    view.render_project_list(
        &project_root_path.display().to_string(),
        &project_hash,
        &summaries,
    )?;

    Ok(())
}
