use crate::args::{OutputFormat, ViewModeArgs};
use agtrace_runtime::AgTrace;
use anyhow::Result;

pub fn handle(
    workspace: &AgTrace,
    _provider_filter: String,
    _verbose: bool,
    format: OutputFormat,
    view_mode: &ViewModeArgs,
) -> Result<()> {
    use crate::presentation::presenters;
    use crate::presentation::{ConsoleRenderer, Renderer};

    let results = workspace.diagnose()?;

    let view_model = presenters::present_diagnose_results(results);

    let presentation_format = crate::presentation::OutputFormat::from(format);
    let renderer = ConsoleRenderer::new(presentation_format, view_mode.resolve());
    renderer.render(view_model)?;

    Ok(())
}
