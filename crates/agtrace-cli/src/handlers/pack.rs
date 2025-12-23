use crate::args::{OutputFormat, ViewModeArgs};
use crate::presentation::v2::presenters;
use crate::presentation::v2::view_models::{CommandResultViewModel, ReportTemplate};
use crate::presentation::v2::{ConsoleRenderer, Renderer};
use agtrace_runtime::AgTrace;
use agtrace_types::resolve_effective_project_hash;
use anyhow::Result;

pub fn handle(
    workspace: &AgTrace,
    template: &str,
    limit: usize,
    project_hash: Option<String>,
    all_projects: bool,
    output_format: OutputFormat,
    view_mode_args: &ViewModeArgs,
) -> Result<()> {
    let (effective_hash_string, _all_projects) =
        resolve_effective_project_hash(project_hash.as_deref(), all_projects)?;
    let effective_project_hash = effective_hash_string.as_deref();

    let result = workspace
        .sessions()
        .pack_context(effective_project_hash, limit)?;

    let report_template: ReportTemplate = template
        .parse()
        .expect("ReportTemplate parsing is infallible");

    let vm = presenters::present_pack_report(
        result.selections,
        report_template,
        result.balanced_count,
        result.raw_count,
    );
    let result = CommandResultViewModel::new(vm);
    let resolved_view_mode = view_mode_args.resolve();
    let renderer = ConsoleRenderer::new(output_format.into(), resolved_view_mode);
    renderer.render(result)?;

    Ok(())
}
