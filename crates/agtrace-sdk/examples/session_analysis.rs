//! Session analysis example: Analyze a specific agent session
//!
//! This example demonstrates:
//! - Getting events from a specific session
//! - Assembling events into a structured session
//! - Analyzing session with diagnostic lenses
//! - Displaying analysis results

use agtrace_sdk::{Client, Lens, analyze_session, assemble_session};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== agtrace SDK: Session Analysis Example ===\n");

    // 1. Connect to workspace
    let workspace_path = dirs::home_dir()
        .ok_or("Could not find home directory")?
        .join(".agtrace");

    let client = Client::connect(&workspace_path)?;
    println!("✓ Connected to workspace\n");

    // 2. Get the most recent session
    let sessions = client.list_sessions()?;
    if sessions.is_empty() {
        println!("No sessions found. Start an agent session first.");
        return Ok(());
    }

    let session_id = &sessions[0].id;
    println!("Analyzing session: {}\n", session_id);

    // 3. Get session events
    let session_handle = client.session(session_id);
    let events = session_handle.events()?;
    println!("  Loaded {} events", events.len());

    // 4. Assemble session
    let session = match assemble_session(&events) {
        Some(s) => s,
        None => {
            println!("  Could not assemble session (may be empty or malformed)");
            return Ok(());
        }
    };

    println!("  Assembled session with {} turns", session.turns.len());
    println!();

    // 5. Get session summary
    let summary = session_handle.summary()?;
    println!("Session Summary:");
    println!("  Total events:   {}", summary.event_counts.total);
    println!("  User messages:  {}", summary.event_counts.user_messages);
    println!("  AI responses:   {}", summary.event_counts.assistant_messages);
    println!("  Tool calls:     {}", summary.event_counts.tool_calls);
    println!("  Reasoning:      {}", summary.event_counts.reasoning_blocks);
    println!();

    // 6. Analyze with diagnostic lenses
    println!("Running diagnostic analysis...");
    let report = analyze_session(session)
        .through(Lens::Failures)
        .through(Lens::Loops)
        .through(Lens::Bottlenecks)
        .report()?;

    println!("\nDiagnostic Results:");
    println!("  Health Score: {}/100", report.score);
    println!();

    if report.insights.is_empty() {
        println!("  ✓ No issues detected");
    } else {
        println!("  Issues found:");
        for (idx, insight) in report.insights.iter().enumerate() {
            println!("    [{}] {}", idx + 1, insight);
        }
    }

    Ok(())
}
