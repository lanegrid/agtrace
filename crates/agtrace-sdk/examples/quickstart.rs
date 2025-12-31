//! Quickstart example: Connect and analyze sessions
//!
//! This minimal example demonstrates:
//! - Connecting to an agtrace workspace
//! - Listing sessions
//! - Analyzing a session with diagnostic lenses
//!
//! For live monitoring, see: examples/watch_events.rs
//!
//! Run with: cargo run -p agtrace-sdk --example quickstart

use agtrace_sdk::{types::SessionFilter, Client, Lens};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to the local workspace (uses XDG data directory)
    let client = Client::connect_default().await?;
    println!("✓ Connected to workspace\n");

    // List sessions and analyze the most recent one
    let sessions = client.sessions().list(SessionFilter::all())?;

    if let Some(summary) = sessions.first() {
        println!("Analyzing session: {}", summary.id);
        let handle = client.sessions().get(&summary.id)?;
        let report = handle
            .analyze()?
            .through(Lens::Failures)
            .through(Lens::Loops)
            .report()?;

        println!("  Health: {}/100", report.score);
        if !report.insights.is_empty() {
            println!("  Issues: {}", report.insights.len());
            for insight in report.insights.iter().take(3) {
                println!("    - Turn {}: {}", insight.turn_index + 1, insight.message);
            }
        } else {
            println!("  ✓ No issues detected");
        }
    } else {
        println!("  No sessions found. Start an agent session first.");
    }

    Ok(())
}
