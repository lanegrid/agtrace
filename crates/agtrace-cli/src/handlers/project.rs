use crate::args::{OutputFormat, ViewModeArgs};
use agtrace_sdk::Client;
use agtrace_sdk::types::{discover_project_root, project_hash_from_root};
use anyhow::Result;

pub fn handle(
    client: &Client,
    project_root: Option<String>,
    output_format: OutputFormat,
    view_mode: &ViewModeArgs,
) -> Result<()> {
    use crate::presentation::presenters;
    use crate::presentation::{ConsoleRenderer, Renderer};

    let project_root_path = discover_project_root(project_root.as_deref())?;
    let project_hash = project_hash_from_root(&project_root_path.to_string_lossy());

    let projects = client.projects().list()?;

    let view_model = presenters::present_project_list(
        project_root_path.display().to_string(),
        project_hash.to_string(),
        projects,
    );

    let presentation_format = crate::presentation::OutputFormat::from(output_format);
    let renderer = ConsoleRenderer::new(presentation_format, view_mode.resolve());
    renderer.render(view_model)?;

    Ok(())
}
