use crate::presentation::presenters::{present_lab_stats, ToolCallSample, ToolClassification};
use crate::presentation::renderers::TraceView;
use agtrace_index::Database;
use agtrace_providers::create_provider;
use agtrace_runtime::{LoadOptions, SessionRepository};
use agtrace_types::EventPayload;
use anyhow::Result;
use std::collections::{BTreeMap, HashMap};

pub fn handle(
    db: &Database,
    limit: Option<usize>,
    source: Option<String>,
    view: &dyn TraceView,
) -> Result<()> {
    let sessions = db.list_sessions(None, limit.unwrap_or(100000))?;
    let sessions: Vec<_> = sessions
        .into_iter()
        .filter(|s| {
            if let Some(ref src) = source {
                &s.provider == src
            } else {
                true
            }
        })
        .collect();

    let total_sessions = sessions.len();

    let loader = SessionRepository::new(db);
    let options = LoadOptions::default();

    // Map: Provider -> (ToolName -> (Count, Sample))
    let mut stats: HashMap<String, HashMap<String, (usize, Option<ToolCallSample>)>> =
        HashMap::new();

    for session in &sessions {
        let events = match loader.load_events(&session.id, &options) {
            Ok(events) => events,
            Err(_e) => {
                continue;
            }
        };

        // Build map of tool_call_id -> ToolResult
        let mut tool_results = HashMap::new();
        for event in &events {
            if let EventPayload::ToolResult(result) = &event.payload {
                tool_results.insert(result.tool_call_id, result.output.clone());
            }
        }

        let provider = &session.provider;
        for event in events {
            if let EventPayload::ToolCall(tool_call) = &event.payload {
                let provider_stats = stats.entry(provider.clone()).or_default();
                let tool_entry = provider_stats
                    .entry(tool_call.name.clone())
                    .or_insert((0, None));

                tool_entry.0 += 1;

                // Store first sample if not already stored
                if tool_entry.1.is_none() {
                    let result = tool_results.get(&event.id).cloned();
                    let arguments = serde_json::to_string(&tool_call.arguments)
                        .unwrap_or_else(|_| String::from("(failed to serialize)"));
                    tool_entry.1 = Some(ToolCallSample { arguments, result });
                }
            }
        }
    }

    // Sort providers and tool names for consistent output
    let sorted_stats: BTreeMap<_, _> = stats
        .into_iter()
        .map(|(provider_name, tools)| {
            let sorted_tools: BTreeMap<_, _> = tools.into_iter().collect();

            // Compute classifications for each tool
            let classifications: Vec<ToolClassification> = sorted_tools
                .keys()
                .map(|tool_name| {
                    let (origin, kind) = if let Ok(provider) = create_provider(&provider_name) {
                        provider
                            .classify_tool(tool_name)
                            .map(|(o, k)| (Some(format!("{:?}", o)), Some(format!("{:?}", k))))
                            .unwrap_or((None, None))
                    } else {
                        (None, None)
                    };

                    ToolClassification {
                        tool_name: tool_name.clone(),
                        origin,
                        kind,
                    }
                })
                .collect();

            (provider_name, (sorted_tools, classifications))
        })
        .collect();

    let stats_vm = present_lab_stats(total_sessions, sorted_stats);
    view.render_lab_stats(&stats_vm)?;

    Ok(())
}
