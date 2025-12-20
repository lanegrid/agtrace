use crate::context::ExecutionContext;
use crate::presentation::presenters;
use crate::presentation::renderers::TraceView;
use agtrace_runtime::services::doctor::DoctorService;
use anyhow::Result;

pub fn handle(
    ctx: &ExecutionContext,
    provider_filter: String,
    verbose: bool,
    view: &dyn TraceView,
) -> Result<()> {
    let providers_with_roots = ctx.resolve_providers(&provider_filter)?;

    for (provider, log_root) in &providers_with_roots {
        if !log_root.exists() {
            view.render_warning(&format!(
                "Warning: log_root does not exist for {}: {}",
                provider.name(),
                log_root.display()
            ))?;
        }
    }

    let results = DoctorService::diagnose_all(&providers_with_roots)?;

    let result_vms = presenters::present_diagnose_results(&results);
    view.render_diagnose_results(&result_vms, verbose)?;

    Ok(())
}
