use crate::presentation::presenters;
use crate::presentation::renderers::TraceView;
use agtrace_runtime::AgTrace;
use anyhow::Result;

pub fn handle(
    workspace: &AgTrace,
    _provider_filter: String,
    verbose: bool,
    view: &dyn TraceView,
) -> Result<()> {
    let results = workspace.diagnose()?;

    let result_vms = presenters::present_diagnose_results(&results);
    view.render_diagnose_results(&result_vms, verbose)?;

    Ok(())
}
