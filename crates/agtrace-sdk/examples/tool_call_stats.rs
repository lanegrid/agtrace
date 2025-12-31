//! Tool call statistics example: Analyze tool usage across all sessions
//!
//! This example demonstrates:
//! - Iterating through all sessions in the workspace
//! - Extracting tool calls from session events
//! - Computing statistics for different tool types (files, MCP, etc.)
//! - Displaying top 5 patterns for each category
//! - Breaking down detailed statistics by provider (claude, codex, gemini)
//!
//! Run with: cargo run -p agtrace-sdk --example tool_call_stats

use agtrace_sdk::{
    Client,
    types::{SessionFilter, ToolCallPayload},
};
use std::collections::HashMap;

/// Statistics for a single provider
#[derive(Default)]
struct ProviderStats {
    total_calls: usize,
    tool_kinds: HashMap<String, usize>,
    tool_names: HashMap<String, usize>,
    file_paths: HashMap<String, usize>,
    mcp_servers: HashMap<String, usize>,
    mcp_tools: HashMap<String, usize>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== agtrace SDK: Tool Call Statistics ===\n");

    // 1. Connect to workspace
    let client = Client::connect_default().await?;
    println!("✓ Connected to workspace\n");

    // 2. Get all sessions
    let sessions = client.sessions().list(SessionFilter::all())?;
    if sessions.is_empty() {
        println!("No sessions found. Start an agent session first.");
        return Ok(());
    }

    println!("Analyzing {} sessions...\n", sessions.len());

    // 3. Collect tool call statistics per provider
    let mut provider_stats: HashMap<String, ProviderStats> = HashMap::new();
    let mut total_tool_calls = 0;

    for session_summary in &sessions {
        let session_handle = client.sessions().get(&session_summary.id)?;
        let provider = &session_summary.provider;

        // Get or create stats for this provider
        let stats = provider_stats.entry(provider.clone()).or_default();

        // Extract tool calls from assembled session
        if let Ok(session) = session_handle.assemble() {
            for turn in &session.turns {
                for step in &turn.steps {
                    for tool_exec in &step.tools {
                        total_tool_calls += 1;
                        stats.total_calls += 1;
                        let call = &tool_exec.call.content;

                        // Count normalized tool kinds
                        let kind = format!("{:?}", call.kind());
                        *stats.tool_kinds.entry(kind).or_insert(0) += 1;

                        // Count raw tool names
                        *stats.tool_names.entry(call.name().to_string()).or_insert(0) += 1;

                        // Categorize by payload type
                        match call {
                            ToolCallPayload::FileRead { arguments, .. } => {
                                if let Some(path) = arguments.path() {
                                    *stats.file_paths.entry(path.to_string()).or_insert(0) += 1;
                                }
                            }
                            ToolCallPayload::FileEdit { arguments, .. } => {
                                *stats
                                    .file_paths
                                    .entry(arguments.file_path.clone())
                                    .or_insert(0) += 1;
                            }
                            ToolCallPayload::FileWrite { arguments, .. } => {
                                *stats
                                    .file_paths
                                    .entry(arguments.file_path.clone())
                                    .or_insert(0) += 1;
                            }
                            ToolCallPayload::Mcp { arguments, .. } => {
                                if let Some(server) = &arguments.server {
                                    *stats.mcp_servers.entry(server.clone()).or_insert(0) += 1;
                                }
                                if let Some(tool) = &arguments.tool {
                                    *stats.mcp_tools.entry(tool.clone()).or_insert(0) += 1;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    // 4. Display statistics
    println!("Total tool calls: {}\n", total_tool_calls);

    // Sort providers by tool call count
    let mut providers: Vec<_> = provider_stats.iter().collect();
    providers.sort_by(|a, b| b.1.total_calls.cmp(&a.1.total_calls));

    // Display statistics for each provider
    for (provider_name, stats) in providers {
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!(
            "Provider: {} (× {} tool calls)",
            provider_name, stats.total_calls
        );
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!();

        // Tool kinds (normalized categories)
        println!("  Tool Kinds (Normalized):");
        print_all_sorted_indented(&stats.tool_kinds);
        println!();

        // Top 5 tool names (raw)
        println!("  Top 5 Tool Names (Raw):");
        print_top_5_indented(&stats.tool_names);
        println!();

        // Top 5 file paths
        if !stats.file_paths.is_empty() {
            println!("  Top 5 File Paths:");
            print_top_5_indented(&stats.file_paths);
            println!();
        }

        // Top 5 MCP servers
        if !stats.mcp_servers.is_empty() {
            println!("  Top 5 MCP Servers:");
            print_top_5_indented(&stats.mcp_servers);
            println!();
        }

        // Top 5 MCP tools
        if !stats.mcp_tools.is_empty() {
            println!("  Top 5 MCP Tools:");
            print_top_5_indented(&stats.mcp_tools);
            println!();
        }
    }

    Ok(())
}

/// Print top 5 items from a frequency map with indentation
fn print_top_5_indented(map: &HashMap<String, usize>) {
    let mut items: Vec<_> = map.iter().collect();
    items.sort_by(|a, b| b.1.cmp(a.1));

    for (i, (key, count)) in items.iter().take(5).enumerate() {
        println!("    {}. {} (× {})", i + 1, key, count);
    }
}

/// Print all items sorted by frequency with indentation
fn print_all_sorted_indented(map: &HashMap<String, usize>) {
    let mut items: Vec<_> = map.iter().collect();
    items.sort_by(|a, b| b.1.cmp(a.1));

    for (key, count) in items {
        println!("    {} (× {})", key, count);
    }
}
