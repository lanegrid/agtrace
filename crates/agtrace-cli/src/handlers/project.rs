use crate::args::OutputFormat;
use agtrace_runtime::AgTrace;
use agtrace_types::discover_project_root;
use anyhow::Result;

pub fn handle_v2(
    workspace: &AgTrace,
    project_root: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    use crate::presentation::v2::presenters;
    use crate::presentation::v2::{ConsoleRenderer, Renderer};

    let project_root_path = discover_project_root(project_root.as_deref())?;
    let project_hash = agtrace_types::project_hash_from_root(&project_root_path.to_string_lossy());

    let projects = workspace.projects().list()?;

    let view_model = presenters::present_project_list(
        project_root_path.display().to_string(),
        project_hash,
        projects,
    );

    let v2_format = crate::presentation::v2::OutputFormat::from(output_format);
    let renderer = ConsoleRenderer::new(v2_format, crate::presentation::v2::ViewMode::default());
    renderer.render(view_model)?;

    Ok(())
}
