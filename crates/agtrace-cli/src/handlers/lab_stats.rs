use crate::ui::TraceView;
use agtrace_index::Database;
use agtrace_providers::create_provider;
use agtrace_runtime::{LoadOptions, SessionRepository};
use agtrace_types::EventPayload;
use anyhow::Result;
use std::collections::{BTreeMap, HashMap};

#[derive(Clone)]
struct ToolCallSample {
    arguments: String,
    result: Option<String>,
}

pub fn handle(
    db: &Database,
    limit: Option<usize>,
    source: Option<String>,
    _view: &dyn TraceView,
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

    println!("Analyzing {} sessions...", sessions.len());

    let loader = SessionRepository::new(db);
    let options = LoadOptions::default();

    // Map: Provider -> (ToolName -> (Count, Sample))
    let mut stats: HashMap<String, HashMap<String, (usize, Option<ToolCallSample>)>> =
        HashMap::new();

    for session in &sessions {
        let events = match loader.load_events(&session.id, &options) {
            Ok(events) => events,
            Err(e) => {
                eprintln!("Warning: Failed to load session {}: {}", session.id, e);
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
        .map(|(provider, tools)| {
            let sorted_tools: BTreeMap<_, _> = tools.into_iter().collect();
            (provider, sorted_tools)
        })
        .collect();

    println!("\n=== ToolCall Statistics by Provider ===");
    for (provider, tools) in &sorted_stats {
        println!("\n{}", "=".repeat(80));
        println!("Provider: {}", provider);
        println!("{}", "=".repeat(80));
        for (tool_name, (count, sample)) in tools {
            println!("\n  Tool: {} (count: {})", tool_name, count);
            if let Some(sample) = sample {
                println!("    Input:");
                println!("      {}", truncate_text(&sample.arguments, 200));
                if let Some(result) = &sample.result {
                    println!("    Output:");
                    println!("      {}", truncate_text(result, 200));
                } else {
                    println!("    Output: (no result found)");
                }
            }
        }
    }

    // Show tool mapping verification
    println!("\n\n{}", "=".repeat(80));
    println!("=== Tool Name → Classification Mapping ===");
    println!("{}", "=".repeat(80));
    print_tool_mappings(&sorted_stats)?;

    Ok(())
}

fn print_tool_mappings(
    stats: &BTreeMap<String, BTreeMap<String, (usize, Option<ToolCallSample>)>>,
) -> Result<()> {
    for (provider_name, tools) in stats {
        println!("\nProvider: {}", provider_name);
        println!("{}", "-".repeat(80));

        let provider = create_provider(provider_name)?;

        // Collect tool names
        let tool_names: Vec<_> = tools.keys().collect();

        // Find max tool name length for alignment
        let max_len = tool_names.iter().map(|n| n.len()).max().unwrap_or(0);

        for tool_name in tool_names {
            let classification = provider.classify_tool(tool_name);
            match classification {
                Some((origin, kind)) => {
                    println!(
                        "  {:width$} → Origin: {:6} Kind: {:8}",
                        tool_name,
                        format!("{:?}", origin),
                        format!("{:?}", kind),
                        width = max_len
                    );
                }
                None => {
                    println!(
                        "  {:width$} → (unmapped - fallback to common classifier)",
                        tool_name,
                        width = max_len
                    );
                }
            }
        }
    }

    Ok(())
}

fn truncate_text(text: &str, max_len: usize) -> String {
    let text = text.replace('\n', " ");
    if text.len() <= max_len {
        text
    } else {
        format!("{}...", &text[..max_len])
    }
}
