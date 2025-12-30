//! Real-time event watching example
//!
//! This example demonstrates:
//! - Setting up a live event stream
//! - Watching for events from all providers
//! - Displaying events as they arrive in real-time
//!
//! NOTE: This will run until Ctrl+C is pressed.
//! Start an agent session in another terminal to see events appear.

use agtrace_sdk::Client;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== agtrace SDK: Real-time Event Watching Example ===\n");

    // 1. Connect to workspace
    let workspace_path = dirs::home_dir()
        .ok_or("Could not find home directory")?
        .join(".agtrace");

    let client = Client::connect(&workspace_path)?;
    println!("✓ Connected to workspace\n");

    // 2. Start watching for events from all providers
    println!("Watching for agent activity...");
    println!("(Start an agent session in another terminal to see events)\n");
    println!("Press Ctrl+C to exit\n");

    let stream = client.watch().all_providers().start()?;

    let mut event_count = 0;
    let start_time = std::time::Instant::now();

    // 3. Process events as they arrive
    loop {
        // Try to get next event without blocking
        if let Some(workspace_event) = stream.try_next() {
            use agtrace_sdk::watch::WorkspaceEvent;

            match workspace_event {
                WorkspaceEvent::Discovery(discovery) => {
                    use agtrace_sdk::watch::DiscoveryEvent;
                    match discovery {
                        DiscoveryEvent::NewSession { summary } => {
                            println!("─────────────────────────────────────────────");
                            println!("📍 New session discovered!");
                            println!("   Session ID: {}", summary.id);
                            println!("   Provider:   {}", summary.provider);
                            println!("─────────────────────────────────────────────");
                        }
                        DiscoveryEvent::SessionUpdated { session_id, provider_name, is_new, .. } => {
                            if is_new {
                                println!("✨ Session started: {} ({})", session_id, provider_name);
                            }
                        }
                        DiscoveryEvent::SessionRemoved { session_id } => {
                            println!("🗑️  Session removed: {}", session_id);
                        }
                    }
                    event_count += 1;
                }
                WorkspaceEvent::Stream(stream_event) => {
                    use agtrace_sdk::watch::StreamEvent;
                    match stream_event {
                        StreamEvent::Attached { session_id, .. } => {
                            println!("🔗 Attached to session: {}", session_id);
                        }
                        StreamEvent::Events { events, .. } => {
                            println!("  📦 Received {} new event(s)", events.len());
                            event_count += events.len();
                        }
                        StreamEvent::Disconnected { reason } => {
                            println!("🔌 Disconnected: {}", reason);
                        }
                    }
                }
                WorkspaceEvent::Error(err) => {
                    println!("❌ Error: {}", err);
                }
            }
        }

        // Small sleep to avoid busy-waiting
        std::thread::sleep(Duration::from_millis(200));

        // Show activity indicator every 10 seconds
        let elapsed = start_time.elapsed().as_secs();
        if event_count == 0 && elapsed > 0 && elapsed % 10 == 0 {
            eprintln!("  (Waiting for events... {} seconds elapsed)", elapsed);
            std::thread::sleep(Duration::from_millis(800)); // Avoid spamming
        }
    }
}
