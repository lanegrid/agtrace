use crate::presentation::view_models::{PackReportViewModel, ReportTemplate, SessionDigest};
use agtrace_sdk::types::SessionDigest as EngineDigest;

pub fn present_pack_report(
    digests: Vec<EngineDigest>,
    template: ReportTemplate,
    pool_size: usize,
    candidate_count: usize,
) -> PackReportViewModel {
    let digest_vms = digests
        .into_iter()
        .map(|digest| SessionDigest {
            session_id: digest.session_id,
            provider: digest.provider,
            opening: digest.opening,
            activation: digest.activation,
            tool_calls_total: digest.metrics.tool_calls_total,
            tool_failures_total: digest.metrics.tool_failures_total,
            max_e2e_ms: digest.metrics.max_e2e_ms,
            max_tool_ms: digest.metrics.max_tool_ms,
            missing_tool_pairs: digest.metrics.missing_tool_pairs,
            loop_signals: digest.metrics.loop_signals,
            longest_chain: digest.metrics.longest_chain,
            recency_boost: digest.recency_boost,
            selection_reason: digest.selection_reason,
        })
        .collect();

    PackReportViewModel {
        template,
        pool_size,
        candidate_count,
        digests: digest_vms,
    }
}
