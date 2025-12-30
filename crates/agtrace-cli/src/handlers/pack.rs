use crate::args::{OutputFormat, ViewModeArgs};
use crate::presentation::presenters;
use crate::presentation::view_models::{CommandResultViewModel, ReportTemplate};
use crate::presentation::{ConsoleRenderer, Renderer};
use agtrace_sdk::Client;
use agtrace_sdk::utils::resolve_effective_project_hash;
use anyhow::Result;

pub fn handle(
    client: &Client,
    template: &str,
    limit: usize,
    project_hash: Option<agtrace_sdk::types::ProjectHash>,
    all_projects: bool,
    output_format: OutputFormat,
    view_mode_args: &ViewModeArgs,
) -> Result<()> {
    let (effective_project_hash, _all_projects) =
        resolve_effective_project_hash(project_hash.as_ref(), all_projects)?;

    let result = client
        .sessions()
        .pack_context(effective_project_hash.as_ref(), limit)?;

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
