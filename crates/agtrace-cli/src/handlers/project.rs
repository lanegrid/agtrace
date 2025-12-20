use crate::context::ExecutionContext;
use crate::presentation::view_models::ProjectSummary;
use crate::presentation::renderers::TraceView;
use agtrace_types::discover_project_root;
use anyhow::Result;

pub fn handle(
    ctx: &ExecutionContext,
    project_root: Option<String>,
    view: &dyn TraceView,
) -> Result<()> {
    let db = ctx.db()?;
    let project_root_path = discover_project_root(project_root.as_deref())?;
    let project_hash = agtrace_types::project_hash_from_root(&project_root_path.to_string_lossy());

    let projects = db.list_projects()?;
    let mut summaries = Vec::new();
    for project in projects {
        let session_count = db.count_sessions_for_project(&project.hash)?;
        summaries.push(ProjectSummary {
            hash: project.hash,
            root_path: project.root_path,
            session_count,
            last_scanned: project.last_scanned_at,
        });
    }

    view.render_project_list(
        &project_root_path.display().to_string(),
        &project_hash,
        &summaries,
    )?;

    Ok(())
}
