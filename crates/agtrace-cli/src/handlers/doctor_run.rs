use crate::context::ExecutionContext;
use crate::presentation::presenters;
use crate::presentation::renderers::TraceView;
use anyhow::Result;

pub fn handle(
    ctx: &ExecutionContext,
    _provider_filter: String,
    verbose: bool,
    view: &dyn TraceView,
) -> Result<()> {
    let workspace = ctx.workspace()?;

    let results = workspace.diagnose()?;

    let result_vms = presenters::present_diagnose_results(&results);
    view.render_diagnose_results(&result_vms, verbose)?;

    Ok(())
}
