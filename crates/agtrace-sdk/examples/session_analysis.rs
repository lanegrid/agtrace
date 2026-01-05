//! Session analysis example: Run diagnostics on an agent session
//!
//! This example demonstrates:
//! - Getting events from a specific session
//! - Assembling events into a structured session
//! - Running diagnostic checks (Failures, Loops, Bottlenecks)
//! - Displaying diagnostic results

use agtrace_sdk::{Client, Diagnostic};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== agtrace SDK: Session Analysis Example ===\n");

    // 1. Connect to workspace (uses system path resolution)
    let client = Client::connect_default().await?;
    println!("✓ Connected to workspace\n");

    // 2. Get the most recent session
    use agtrace_sdk::types::SessionFilter;
    let sessions = client.sessions().list(SessionFilter::all())?;
    if sessions.is_empty() {
        println!("No sessions found. Start an agent session first.");
        return Ok(());
    }

    let session_id = &sessions[0].id;
    println!("Analyzing session: {}\n", session_id);

    // 3. Get session handle and assemble events
    let session_handle = client.sessions().get(session_id)?;
    let events = session_handle.events()?;
    println!("  Loaded {} events", events.len());

    // 4. Assemble session
    let session = match session_handle.assemble() {
        Ok(s) => s,
        Err(_) => {
            println!("  Could not assemble session (may be empty or malformed)");
            return Ok(());
        }
    };

    println!("  Assembled session with {} turns", session.turns.len());
    println!();

    // 5. Get session summary
    let summary = session_handle.summarize()?;
    println!("Session Summary:");
    println!("  Total events:   {}", summary.event_counts.total);
    println!("  User messages:  {}", summary.event_counts.user_messages);
    println!(
        "  AI responses:   {}",
        summary.event_counts.assistant_messages
    );
    println!("  Tool calls:     {}", summary.event_counts.tool_calls);
    println!(
        "  Reasoning:      {}",
        summary.event_counts.reasoning_blocks
    );
    println!();

    // 6. Run diagnostic checks
    println!("Running diagnostics...");
    let report = session_handle
        .analyze()?
        .check(Diagnostic::Failures)
        .check(Diagnostic::Loops)
        .check(Diagnostic::Bottlenecks)
        .report()?;

    println!("\nDiagnostic Results:");
    println!("  Health Score: {}/100", report.score);
    println!();

    if report.insights.is_empty() {
        println!("  No issues detected");
    } else {
        println!("  Issues found:");
        for insight in &report.insights {
            let severity_icon = match insight.severity {
                agtrace_sdk::Severity::Info => "i",
                agtrace_sdk::Severity::Warning => "!",
                agtrace_sdk::Severity::Critical => "X",
            };
            let diagnostic_label = match insight.diagnostic {
                agtrace_sdk::Diagnostic::Failures => "Failure",
                agtrace_sdk::Diagnostic::Loops => "Loop",
                agtrace_sdk::Diagnostic::Bottlenecks => "Bottleneck",
            };
            println!(
                "    [{}] Turn {}: [{}] {}",
                severity_icon,
                insight.turn_index + 1,
                diagnostic_label,
                insight.message
            );
        }
    }

    Ok(())
}
