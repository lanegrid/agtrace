use std::collections::BTreeMap;
use std::path::Path;

use crate::presentation::v2::view_models::{
    LabExportViewModel, LabStatsViewModel, ProviderStats, ToolCallSample, ToolClassification,
    ToolStatsEntry,
};

pub fn present_lab_export(exported_count: usize, output_path: &Path) -> LabExportViewModel {
    LabExportViewModel {
        exported_count,
        output_path: output_path.display().to_string(),
    }
}

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
    stats: BTreeMap<String, (BTreeMap<String, (usize, Option<ToolCallSample>)>, Vec<ToolClassification>)>,
) -> LabStatsViewModel {
    let providers: Vec<ProviderStats> = stats
        .into_iter()
        .map(|(provider_name, (tools, classifications))| {
            let tool_entries: Vec<ToolStatsEntry> = tools
                .iter()
                .map(|(tool_name, (count, sample))| ToolStatsEntry {
                    tool_name: tool_name.clone(),
                    count: *count,
                    sample: sample.as_ref().map(|s| ToolCallSample {
                        arguments: truncate_text(&s.arguments, 200),
                        result: s.result.as_ref().map(|r| truncate_text(r, 200)),
                    }),
                })
                .collect();

            ProviderStats {
                provider_name,
                tools: tool_entries,
                classifications,
            }
        })
        .collect();

    LabStatsViewModel {
        total_sessions,
        providers,
    }
}
