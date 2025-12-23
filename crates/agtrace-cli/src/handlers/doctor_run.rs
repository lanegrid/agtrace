use crate::args::{OutputFormat, ViewModeArgs};
use agtrace_runtime::AgTrace;
use anyhow::Result;

pub fn handle_v2(
    workspace: &AgTrace,
    _provider_filter: String,
    _verbose: bool,
    format: OutputFormat,
    view_mode: &ViewModeArgs,
) -> Result<()> {
    use crate::presentation::v2::presenters;
    use crate::presentation::v2::{ConsoleRenderer, Renderer};

    let results = workspace.diagnose()?;

    let view_model = presenters::present_diagnose_results(results);

    let v2_format = crate::presentation::v2::OutputFormat::from(format);
    let renderer = ConsoleRenderer::new(v2_format, view_mode.resolve());
    renderer.render(view_model)?;

    Ok(())
}
