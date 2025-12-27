use crate::args::{OutputFormat, ViewModeArgs};
use crate::presentation::presenters;
use crate::presentation::view_models::{
    CommandResultViewModel, ToolCallSample, ToolClassification,
};
use crate::presentation::{ConsoleRenderer, Renderer};
use agtrace_runtime::AgTrace;
use anyhow::Result;

pub fn handle(
    workspace: &AgTrace,
    limit: Option<usize>,
    provider: Option<String>,
    output_format: OutputFormat,
    view_mode_args: &ViewModeArgs,
) -> Result<()> {
    let result = workspace.insights().tool_usage(limit, provider)?;

    let sorted_stats = result
        .provider_stats
        .into_iter()
        .map(|(provider_name, (tools, classifications))| {
            let tool_samples = tools
                .into_iter()
                .map(|(name, (count, sample))| {
                    let sample = sample.map(|s| ToolCallSample {
                        arguments: s.arguments,
                        result: s.result,
                    });
                    (name, (count, sample))
                })
                .collect();

            let tool_classifications = classifications
                .into_iter()
                .map(|info| ToolClassification {
                    tool_name: info.tool_name,
                    origin: info.origin,
                    kind: info.kind,
                })
                .collect();

            (provider_name, (tool_samples, tool_classifications))
        })
        .collect();

    let vm = presenters::present_lab_stats(result.total_sessions, sorted_stats);
    let result_vm = CommandResultViewModel::new(vm);
    let resolved_view_mode = view_mode_args.resolve();
    let renderer = ConsoleRenderer::new(output_format.into(), resolved_view_mode);
    renderer.render(result_vm)?;

    Ok(())
}
