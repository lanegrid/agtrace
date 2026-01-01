//! Token monotonicity verification example
//!
//! This example verifies that token usage is monotonically increasing across turns.
//! It detects anomalies where cumulative tokens decrease or fluctuate unexpectedly.
//!
//! Run with: cargo run --release -p agtrace-sdk --example token_monotonicity

use agtrace_sdk::{Client, types::SessionFilter};
use std::collections::HashMap;

#[derive(Debug)]
struct TokenAnomaly {
    turn_index: usize,
    previous_total: u64,
    current_total: u64,
    decrease_amount: i64,
}

#[derive(Debug)]
struct SessionTokenAnalysis {
    session_id: String,
    provider: String,
    total_turns: usize,
    anomalies: Vec<TokenAnomaly>,
    is_monotonic: bool,
    token_sequence: Vec<u64>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== agtrace SDK: Token Monotonicity Verification ===\n");

    // Connect to workspace
    let client = Client::connect_default().await?;
    println!("✓ Connected to workspace\n");

    // Get all sessions
    let sessions = client.sessions().list(SessionFilter::all())?;
    if sessions.is_empty() {
        println!("No sessions found. Start an agent session first.");
        return Ok(());
    }

    println!(
        "Analyzing {} sessions for token monotonicity...\n",
        sessions.len()
    );

    let mut all_analyses: Vec<SessionTokenAnalysis> = Vec::new();
    let mut provider_stats: HashMap<String, (usize, usize)> = HashMap::new(); // (total, anomalies)

    // Analyze each session
    for session_summary in &sessions {
        if let Ok(session_handle) = client.sessions().get(&session_summary.id) {
            if let Ok(session) = session_handle.assemble() {
                let mut token_sequence = Vec::new();
                let mut anomalies = Vec::new();
                let mut prev_total: Option<u64> = None;

                // Collect cumulative token usage across turns
                for (turn_idx, turn) in session.turns.iter().enumerate() {
                    // Get cumulative tokens from the last step's usage (which contains cumulative totals)
                    let cumulative = turn
                        .steps
                        .iter()
                        .rev()
                        .find_map(|step| step.usage.as_ref())
                        .copied()
                        .unwrap_or_default();
                    let current_total = cumulative.total_tokens().as_u64();
                    token_sequence.push(current_total);

                    if let Some(prev) = prev_total {
                        if current_total < prev {
                            // Detected decrease in cumulative tokens
                            anomalies.push(TokenAnomaly {
                                turn_index: turn_idx,
                                previous_total: prev,
                                current_total,
                                decrease_amount: current_total as i64 - prev as i64,
                            });
                        }
                    }

                    prev_total = Some(current_total);
                }

                let is_monotonic = anomalies.is_empty();

                // Update provider stats
                let stats = provider_stats
                    .entry(session_summary.provider.clone())
                    .or_insert((0, 0));
                stats.0 += 1;
                if !is_monotonic {
                    stats.1 += 1;
                }

                all_analyses.push(SessionTokenAnalysis {
                    session_id: session_summary.id.clone(),
                    provider: session_summary.provider.clone(),
                    total_turns: session.turns.len(),
                    anomalies,
                    is_monotonic,
                    token_sequence,
                });
            }
        }
    }

    // Summary statistics
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("SUMMARY");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let total_sessions = all_analyses.len();
    let sessions_with_anomalies = all_analyses.iter().filter(|a| !a.is_monotonic).count();
    let total_anomalies: usize = all_analyses.iter().map(|a| a.anomalies.len()).sum();

    println!("Total sessions analyzed:    {}", total_sessions);
    println!(
        "Sessions with anomalies:    {} ({:.1}%)",
        sessions_with_anomalies,
        if total_sessions > 0 {
            (sessions_with_anomalies as f64 / total_sessions as f64) * 100.0
        } else {
            0.0
        }
    );
    println!("Total anomalies detected:   {}", total_anomalies);

    if sessions_with_anomalies == 0 {
        println!("\n✅ All sessions have monotonically increasing token usage!");
    } else {
        println!("\n⚠️  Token monotonicity violations detected!");
    }
    println!();

    // Provider breakdown
    if !provider_stats.is_empty() {
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("PROVIDER BREAKDOWN");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

        println!(
            "{:<15} {:>12} {:>15} {:>12}",
            "Provider", "Total", "With Anomalies", "Anomaly %"
        );
        println!("{}", "─".repeat(60));

        let mut providers: Vec<_> = provider_stats.iter().collect();
        providers.sort_by(|a, b| b.1.0.cmp(&a.1.0));

        for (provider, (total, anomalies)) in providers {
            let anomaly_pct = if *total > 0 {
                (*anomalies as f64 / *total as f64) * 100.0
            } else {
                0.0
            };
            println!(
                "{:<15} {:>12} {:>15} {:>11.1}%",
                provider, total, anomalies, anomaly_pct
            );
        }
        println!();
    }

    // Detailed anomaly reports
    let sessions_with_issues: Vec<_> = all_analyses.iter().filter(|a| !a.is_monotonic).collect();

    if !sessions_with_issues.is_empty() {
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("DETAILED ANOMALY REPORTS");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

        for analysis in sessions_with_issues.iter().take(10) {
            println!("Session: {} ({})", analysis.session_id, analysis.provider);
            println!("Total turns: {}", analysis.total_turns);
            println!("Anomalies: {}", analysis.anomalies.len());
            println!();

            println!("Token sequence:");
            for (idx, tokens) in analysis.token_sequence.iter().enumerate() {
                let marker = if analysis.anomalies.iter().any(|a| a.turn_index == idx) {
                    " ⚠️"
                } else {
                    ""
                };
                println!("  Turn {}: {:>10} tokens{}", idx, tokens, marker);
            }
            println!();

            println!("Detected anomalies:");
            for anomaly in &analysis.anomalies {
                println!(
                    "  Turn {}: {} → {} (decrease: {})",
                    anomaly.turn_index,
                    anomaly.previous_total,
                    anomaly.current_total,
                    anomaly.decrease_amount
                );
            }
            println!();
            println!("{}", "─".repeat(70));
            println!();
        }

        if sessions_with_issues.len() > 10 {
            println!(
                "... and {} more sessions with anomalies\n",
                sessions_with_issues.len() - 10
            );
        }
    }

    // Analysis and recommendations
    if sessions_with_anomalies > 0 {
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("ANALYSIS");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

        println!(
            "⚠️  Token monotonicity violations detected in {} sessions.\n",
            sessions_with_anomalies
        );

        println!("Potential causes:");
        println!("1. Incorrect cumulative token calculation logic");
        println!("2. Provider-specific event ordering issues");
        println!("3. Missing or duplicate token usage events");
        println!("4. Schema-on-read parsing errors");
        println!();

        println!("Recommended actions:");
        println!("1. Inspect raw logs for affected sessions:");
        println!("   ./target/release/agtrace lab grep '<session_id>' --json --limit 100");
        println!();
        println!("2. Check turn-level token usage structure:");
        println!("   ./target/release/agtrace show <session_id> --json | jq '.turns[].usage'");
        println!();
        println!("3. Verify provider-specific token accumulation logic in:");
        println!("   crates/agtrace-providers/src/<provider>/parser.rs");
        println!();
    }

    Ok(())
}
