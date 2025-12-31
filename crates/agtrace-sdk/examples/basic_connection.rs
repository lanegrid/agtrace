//! Basic SDK example: Connect to agtrace workspace and list sessions
//!
//! This example demonstrates:
//! - Connecting to an agtrace workspace
//! - Listing all sessions for the current project
//! - Displaying basic session information

use agtrace_sdk::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== agtrace SDK: Basic Connection Example ===\n");

    // 1. Connect to the agtrace workspace (uses XDG path resolution)
    println!("Connecting to workspace...");

    let client = Client::connect_default().await?;
    println!("✓ Connected successfully\n");

    // 2. List all sessions for the current project
    println!("Listing sessions...");
    use agtrace_sdk::types::SessionFilter;
    let sessions = client.sessions().list(SessionFilter::default())?;

    if sessions.is_empty() {
        println!("  No sessions found. Run 'agtrace init' and start an agent session first.");
        return Ok(());
    }

    println!("  Found {} session(s):\n", sessions.len());

    // 3. Display session information
    for (idx, summary) in sessions.iter().enumerate().take(10) {
        println!("  [{}] Session ID: {}", idx + 1, summary.id);
        println!("      Provider:   {}", summary.provider);
        if let Some(start_ts) = &summary.start_ts {
            println!("      Started:    {}", start_ts);
        }
        if let Some(snippet) = &summary.snippet {
            let trimmed = snippet.chars().take(60).collect::<String>();
            println!(
                "      Snippet:    {}{}",
                trimmed,
                if snippet.len() > 60 { "..." } else { "" }
            );
        }
        println!();
    }

    if sessions.len() > 10 {
        println!("  ... and {} more session(s)", sessions.len() - 10);
    }

    Ok(())
}
