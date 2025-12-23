use std::collections::BTreeMap;
use std::path::Path;

use crate::presentation::v2::view_models::{
    EventPayloadViewModel, EventViewModel, LabExportViewModel, LabGrepViewModel, LabStatsViewModel,
    ProviderStats, ToolCallSample, ToolClassification, ToolStatsEntry,
};
use agtrace_types::{AgentEvent, EventPayload};

// Type aliases for complex nested types
type ToolStatsMap = BTreeMap<String, (usize, Option<ToolCallSample>)>;
type ProviderStatsData = (ToolStatsMap, Vec<ToolClassification>);
type ProviderStatsMap = BTreeMap<String, ProviderStatsData>;

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

pub fn present_lab_stats(total_sessions: usize, stats: ProviderStatsMap) -> LabStatsViewModel {
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

fn present_event_payload(payload: &EventPayload) -> EventPayloadViewModel {
    match payload {
        EventPayload::User(p) => EventPayloadViewModel::User {
            text: p.text.clone(),
        },
        EventPayload::Reasoning(p) => EventPayloadViewModel::Reasoning {
            text: p.text.clone(),
        },
        EventPayload::ToolCall(p) => {
            let arguments = serde_json::to_value(p)
                .ok()
                .and_then(|v| v.get("arguments").cloned())
                .unwrap_or(serde_json::Value::Null);
            EventPayloadViewModel::ToolCall {
                name: p.name().to_string(),
                arguments,
            }
        }
        EventPayload::ToolResult(p) => EventPayloadViewModel::ToolResult {
            output: p.output.clone(),
            is_error: p.is_error,
        },
        EventPayload::Message(p) => EventPayloadViewModel::Message {
            text: p.text.clone(),
        },
        EventPayload::TokenUsage(p) => EventPayloadViewModel::TokenUsage {
            input: p.input_tokens,
            output: p.output_tokens,
            total: p.total_tokens,
            cache_creation: p
                .details
                .as_ref()
                .and_then(|d| d.cache_creation_input_tokens),
            cache_read: p.details.as_ref().and_then(|d| d.cache_read_input_tokens),
        },
        EventPayload::Notification(p) => EventPayloadViewModel::Notification {
            text: p.text.clone(),
            level: p.level.clone(),
        },
    }
}

pub fn present_event(event: &AgentEvent) -> EventViewModel {
    EventViewModel {
        id: event.id.to_string(),
        session_id: event.session_id.to_string(),
        parent_id: event.parent_id.map(|id| id.to_string()),
        timestamp: event.timestamp,
        stream_id: event.stream_id.clone(),
        payload: present_event_payload(&event.payload),
        metadata: event.metadata.clone(),
    }
}

pub fn present_events(events: &[AgentEvent]) -> Vec<EventViewModel> {
    events.iter().map(present_event).collect()
}

pub fn present_lab_grep(
    pattern: String,
    matches: Vec<EventViewModel>,
    json_output: bool,
) -> LabGrepViewModel {
    LabGrepViewModel {
        pattern,
        matches,
        json_output,
    }
}
