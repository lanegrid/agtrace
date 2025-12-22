use crate::args::OutputFormat;
use agtrace_runtime::AgTrace;
use anyhow::Result;

pub fn handle_v2(
    workspace: &AgTrace,
    _provider_filter: String,
    _verbose: bool,
    format: OutputFormat,
) -> Result<()> {
    use crate::presentation::v2::presenters;
    use crate::presentation::v2::{ConsoleRenderer, Renderer};

    let results = workspace.diagnose()?;

    let view_model = presenters::present_diagnose_results(results);

    let renderer = ConsoleRenderer::new(format == OutputFormat::Json);
    renderer.render(view_model)?;

    Ok(())
}
