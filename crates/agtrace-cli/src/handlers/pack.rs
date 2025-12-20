use crate::context::ExecutionContext;
use crate::presentation::presenters;
use crate::presentation::renderers::TraceView;
use agtrace_types::resolve_effective_project_hash;
use anyhow::Result;

pub fn handle(
    ctx: &ExecutionContext,
    template: &str,
    limit: usize,
    project_hash: Option<String>,
    view: &dyn TraceView,
) -> Result<()> {
    let workspace = ctx.workspace()?;
    let all_projects = ctx.all_projects;
    let (effective_hash_string, _all_projects) =
        resolve_effective_project_hash(project_hash.as_deref(), all_projects)?;
    let effective_project_hash = effective_hash_string.as_deref();

    let result = workspace
        .sessions()
        .pack_context(effective_project_hash, limit)?;

    let report_template = template
        .parse()
        .expect("ReportTemplate parsing is infallible");
    let selection_vms: Vec<_> = result
        .selections
        .iter()
        .map(presenters::present_digest)
        .collect();
    view.render_pack_report(
        &selection_vms,
        report_template,
        result.balanced_count,
        result.raw_count,
    )?;

    Ok(())
}
