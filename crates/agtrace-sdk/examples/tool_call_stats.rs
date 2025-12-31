//! Tool call statistics example: Analyze tool usage across all sessions
//!
//! This example demonstrates:
//! - Iterating through all sessions in the workspace
//! - Extracting tool calls from session events
//! - Computing statistics for different tool types (files, MCP, etc.)
//! - Displaying top 5 patterns for each category
//! - Breaking down statistics by provider (claude, codex, gemini)
//!
//! Run with: cargo run -p agtrace-sdk --example tool_call_stats

use agtrace_sdk::{
    Client,
    types::{SessionFilter, ToolCallPayload},
};
use std::collections::HashMap;

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

    // 3. Collect tool call statistics
    let mut file_paths: HashMap<String, usize> = HashMap::new();
    let mut mcp_servers: HashMap<String, usize> = HashMap::new();
    let mut mcp_tools: HashMap<String, usize> = HashMap::new();
    let mut tool_names: HashMap<String, usize> = HashMap::new();
    let mut provider_stats: HashMap<String, usize> = HashMap::new();

    let mut total_tool_calls = 0;

    for session_summary in &sessions {
        let session_handle = client.sessions().get(&session_summary.id)?;
        let provider = &session_summary.provider;

        // Extract tool calls from assembled session
        if let Ok(session) = session_handle.assemble() {
            for turn in &session.turns {
                for step in &turn.steps {
                    for tool_exec in &step.tools {
                        total_tool_calls += 1;
                        let call = &tool_exec.call.content;

                        // Count by provider
                        *provider_stats.entry(provider.clone()).or_insert(0) += 1;

                        // Count tool names
                        *tool_names.entry(call.name().to_string()).or_insert(0) += 1;

                        // Categorize by payload type
                        match call {
                            ToolCallPayload::FileRead { arguments, .. } => {
                                if let Some(path) = arguments.path() {
                                    *file_paths.entry(path.to_string()).or_insert(0) += 1;
                                }
                            }
                            ToolCallPayload::FileEdit { arguments, .. } => {
                                *file_paths.entry(arguments.file_path.clone()).or_insert(0) += 1;
                            }
                            ToolCallPayload::FileWrite { arguments, .. } => {
                                *file_paths.entry(arguments.file_path.clone()).or_insert(0) += 1;
                            }
                            ToolCallPayload::Mcp { arguments, .. } => {
                                if let Some(server) = &arguments.server {
                                    *mcp_servers.entry(server.clone()).or_insert(0) += 1;
                                }
                                if let Some(tool) = &arguments.tool {
                                    *mcp_tools.entry(tool.clone()).or_insert(0) += 1;
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

    // Provider breakdown
    if !provider_stats.is_empty() {
        println!("Tool Calls by Provider:");
        print_all_sorted(&provider_stats);
        println!();
    }

    // Top 5 tool names
    println!("Top 5 Tool Names:");
    print_top_5(&tool_names);
    println!();

    // Top 5 file paths
    if !file_paths.is_empty() {
        println!("Top 5 File Paths:");
        print_top_5(&file_paths);
        println!();
    }

    // Top 5 MCP servers
    if !mcp_servers.is_empty() {
        println!("Top 5 MCP Servers:");
        print_top_5(&mcp_servers);
        println!();
    }

    // Top 5 MCP tools
    if !mcp_tools.is_empty() {
        println!("Top 5 MCP Tools:");
        print_top_5(&mcp_tools);
        println!();
    }

    Ok(())
}

/// Print top 5 items from a frequency map
fn print_top_5(map: &HashMap<String, usize>) {
    let mut items: Vec<_> = map.iter().collect();
    items.sort_by(|a, b| b.1.cmp(a.1));

    for (i, (key, count)) in items.iter().take(5).enumerate() {
        println!("  {}. {} (× {})", i + 1, key, count);
    }
}

/// Print all items sorted by frequency
fn print_all_sorted(map: &HashMap<String, usize>) {
    let mut items: Vec<_> = map.iter().collect();
    items.sort_by(|a, b| b.1.cmp(a.1));

    for (key, count) in items {
        println!("  {} (× {})", key, count);
    }
}
