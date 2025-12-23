use crate::presentation::v1::view_models::{
    LabStatsViewModel, ProviderStatsViewModel, ToolCallSampleViewModel,
    ToolClassificationViewModel, ToolStatsEntry,
};
use std::collections::BTreeMap;

#[derive(Clone)]
pub struct ToolCallSample {
    pub arguments: String,
    pub result: Option<String>,
}

pub struct ToolClassification {
    pub tool_name: String,
    pub origin: Option<String>,
    pub kind: Option<String>,
}

type ProviderStatsData = (
    BTreeMap<String, (usize, Option<ToolCallSample>)>,
    Vec<ToolClassification>,
);

fn truncate_text(text: &str, max_len: usize) -> String {
    let text = text.replace('\n', " ");
    if text.len() <= max_len {
        text
    } else {
        format!("{}...", &text[..max_len])
    }
}

pub fn present_lab_stats(
    total_sessions: usize,
    stats: BTreeMap<String, ProviderStatsData>,
) -> LabStatsViewModel {
    let providers: Vec<ProviderStatsViewModel> = stats
        .into_iter()
        .map(|(provider_name, (tools, classifications))| {
            let tool_entries: Vec<ToolStatsEntry> = tools
                .iter()
                .map(|(tool_name, (count, sample))| ToolStatsEntry {
                    tool_name: tool_name.clone(),
                    count: *count,
                    sample: sample.as_ref().map(|s| ToolCallSampleViewModel {
                        arguments: truncate_text(&s.arguments, 200),
                        result: s.result.as_ref().map(|r| truncate_text(r, 200)),
                    }),
                })
                .collect();

            let classification_vms: Vec<ToolClassificationViewModel> = classifications
                .into_iter()
                .map(|c| ToolClassificationViewModel {
                    tool_name: c.tool_name,
                    origin: c.origin,
                    kind: c.kind,
                })
                .collect();

            ProviderStatsViewModel {
                provider_name,
                tools: tool_entries,
                classifications: classification_vms,
            }
        })
        .collect();

    LabStatsViewModel {
        total_sessions,
        providers,
    }
}
