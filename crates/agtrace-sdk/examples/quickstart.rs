//! Quickstart example: Connect and browse sessions
//!
//! This minimal example demonstrates:
//! - Connecting to an agtrace workspace
//! - Listing sessions
//! - Browsing structured session data (Turn → Step → Tool)
//!
//! For live monitoring, see: examples/watch_events.rs
//! For diagnostics, see: examples/session_analysis.rs
//!
//! Run with: cargo run -p agtrace-sdk --example quickstart

use agtrace_sdk::{Client, types::SessionFilter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to the local workspace
    let client = Client::connect_default().await?;
    println!("Connected to workspace\n");

    // List sessions
    let sessions = client.sessions().list(SessionFilter::all())?;

    if let Some(summary) = sessions.first() {
        println!("Session: {}", summary.id);
        println!("Provider: {}", summary.provider);

        // Get session handle and assemble structured data
        let handle = client.sessions().get(&summary.id)?;
        let session = handle.assemble()?;

        // Count tool calls
        let tool_count: usize = session.turns.iter()
            .flat_map(|t| &t.steps)
            .flat_map(|s| &s.tools)
            .count();

        println!("\nStats:");
        println!("  Turns: {}", session.turns.len());
        println!("  Tool calls: {}", tool_count);
        println!("  Tokens: {}", session.stats.total_tokens);

        // Browse tool calls
        if !session.turns.is_empty() {
            println!("\nTool calls (first turn):");
            for step in &session.turns[0].steps {
                for tool in &step.tools {
                    let status = if tool.is_error { "failed" } else { "ok" };
                    println!("  {} ({})", tool.call.content.name(), status);
                }
            }
        }
    } else {
        println!("No sessions found. Start an agent session first.");
    }

    Ok(())
}
