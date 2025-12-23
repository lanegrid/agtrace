use crate::presentation::v1::presenters::{present_lab_stats, ToolCallSample, ToolClassification};
use crate::presentation::v1::renderers::TraceView;
use agtrace_runtime::AgTrace;
use anyhow::Result;

pub fn handle(
    workspace: &AgTrace,
    limit: Option<usize>,
    source: Option<String>,
    view: &dyn TraceView,
) -> Result<()> {
    let result = workspace.insights().tool_usage(limit, source)?;

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

    let stats_vm = present_lab_stats(result.total_sessions, sorted_stats);
    view.render_lab_stats(&stats_vm)?;

    Ok(())
}
