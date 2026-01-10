//! Debug example for investigating token usage display issues
//!
//! This example tracks and displays detailed token usage information
//! from watch events to help debug synchronization issues between
//! event updates and percentage calculations.

use agtrace_sdk::Client;
use agtrace_sdk::types::{SessionState, StreamEvent, WorkspaceEvent};
use agtrace_sdk::utils::extract_state_updates;
use std::collections::VecDeque;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Token Usage Debug Example ===\n");

    let client = Client::connect_default().await?;
    println!("✓ Connected to workspace\n");

    // Watch specific provider
    let provider_name = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "claude_code".to_string());

    println!("Watching provider: {}\n", provider_name);

    let watch_service = client.watch_service();
    let builder = watch_service.watch_all_providers()?;
    let monitor = builder.start_background_scan()?;
    let rx_discovery = monitor.receiver();

    let mut current_handle: Option<agtrace_sdk::types::StreamHandle> = None;
    let mut session_state: Option<SessionState> = None;
    let mut event_buffer: VecDeque<agtrace_sdk::types::AgentEvent> = VecDeque::new();

    println!("Waiting for events...\n");

    loop {
        // Check for new session discoveries
        match rx_discovery.try_recv() {
            Ok(WorkspaceEvent::Discovery(disc)) => {
                use agtrace_sdk::types::DiscoveryEvent;
                match disc {
                    DiscoveryEvent::NewSession { summary } => {
                        println!("─────────────────────────────────────────");
                        println!("📍 New session: {}", &summary.id[..8]);
                        println!("─────────────────────────────────────────\n");

                        match watch_service.watch_session(&summary.id) {
                            Ok(handle) => {
                                current_handle = Some(handle);
                                session_state = None;
                                event_buffer.clear();
                            }
                            Err(e) => eprintln!("Failed to attach: {}", e),
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        // Process stream events
        if let Some(ref handle) = current_handle {
            match handle
                .receiver()
                .recv_timeout(std::time::Duration::from_millis(100))
            {
                Ok(WorkspaceEvent::Stream(StreamEvent::Attached { session_id, .. })) => {
                    println!("🔗 Attached to: {}\n", &session_id[..8]);
                    session_state = Some(SessionState::new(
                        session_id,
                        None,
                        None,
                        chrono::Utc::now(),
                    ));
                }
                Ok(WorkspaceEvent::Stream(StreamEvent::Events { events, session })) => {
                    if let Some(state) = &mut session_state {
                        for event in &events {
                            state.last_activity = event.timestamp;
                            state.event_count += 1;

                            let updates = extract_state_updates(event);

                            if updates.is_new_turn {
                                state.turn_count += 1;
                                println!("\n🔄 Turn {} started", state.turn_count);
                            }

                            if let Some(usage) = updates.usage {
                                println!(
                                    "  📊 TokenUsage event: fresh={}, cache={}, output={}, total={}",
                                    usage.fresh_input.0,
                                    usage.cache_read.0,
                                    usage.output.0,
                                    usage.total_tokens().as_u64()
                                );

                                // Show what happens when we update state
                                let old_total = state.current_usage.total_tokens().as_u64();
                                state.current_usage = usage;
                                let new_total = state.current_usage.total_tokens().as_u64();

                                println!(
                                    "  📈 State updated: {} -> {} tokens",
                                    old_total, new_total
                                );

                                if new_total < old_total {
                                    println!("  ⚠️  WARNING: Token count decreased!");
                                }

                                // Show assembled session total if available
                                if let Some(ref session) = session {
                                    let assembled_total: u64 = session
                                        .turns
                                        .iter()
                                        .flat_map(|t| &t.steps)
                                        .filter_map(|s| s.usage.as_ref())
                                        .map(|u| (u.input_tokens() + u.output_tokens()) as u64)
                                        .sum();
                                    println!(
                                        "  🎯 Assembled session total: {} tokens",
                                        assembled_total
                                    );

                                    if assembled_total != new_total {
                                        println!(
                                            "  ⚠️  MISMATCH: state.current_usage ({}) != assembled total ({})",
                                            new_total, assembled_total
                                        );
                                    }
                                }
                            }

                            if let Some(model) = updates.model {
                                if state.model.is_none() {
                                    state.model = Some(model);
                                }
                            }

                            if let Some(limit) = updates.context_window_limit {
                                if state.context_window_limit.is_none() {
                                    state.context_window_limit = Some(limit);
                                    println!("  📏 Context window limit: {} tokens", limit);
                                }
                            }

                            event_buffer.push_back(event.clone());
                            if event_buffer.len() > 100 {
                                event_buffer.pop_front();
                            }
                        }
                    }
                }
                Ok(WorkspaceEvent::Stream(StreamEvent::Disconnected { .. })) => {
                    println!("\n🔌 Disconnected\n");
                    current_handle = None;
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    current_handle = None;
                }
                _ => {}
            }
        } else {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
}
