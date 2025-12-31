//! Provider efficiency comparison example
//!
//! This example demonstrates:
//! - Computing efficiency metrics for each provider (Claude Code, Codex, Gemini)
//! - Measuring tool call parallelization, diversity, and success rates
//! - Analyzing implementation styles (Read/Write/Execute ratios)
//! - Comparing token efficiency and session productivity
//!
//! Efficiency Metrics:
//! 1. Tools per session: Average number of tool calls per session
//! 2. Parallelization rate: % of turns with multiple parallel tool calls
//! 3. Tool diversity: Shannon entropy of tool usage distribution
//! 4. Read/Write/Execute ratio: Implementation style analysis
//! 5. Error rate: % of failed tool calls
//!
//! Run with: cargo run --release -p agtrace-sdk --example provider_efficiency

use agtrace_sdk::{
    Client,
    types::{SessionFilter, ToolKind},
};
use std::collections::HashMap;

#[derive(Default)]
struct ProviderEfficiency {
    // Basic counts
    total_sessions: usize,
    total_tool_calls: usize,
    total_turns: usize,
    total_steps: usize,

    // Parallelization metrics
    turns_with_multiple_tools: usize,
    parallel_tool_calls: usize,

    // Tool kind distribution
    read_calls: usize,
    write_calls: usize,
    execute_calls: usize,
    search_calls: usize,
    other_calls: usize,

    // Tool diversity
    tool_name_counts: HashMap<String, usize>,

    // Error tracking
    failed_tool_calls: usize,

    // Token metrics (if available)
    total_tokens: usize,
}

impl ProviderEfficiency {
    fn tools_per_session(&self) -> f64 {
        if self.total_sessions == 0 {
            0.0
        } else {
            self.total_tool_calls as f64 / self.total_sessions as f64
        }
    }

    fn parallelization_rate(&self) -> f64 {
        if self.total_turns == 0 {
            0.0
        } else {
            (self.turns_with_multiple_tools as f64 / self.total_turns as f64) * 100.0
        }
    }

    fn parallel_calls_ratio(&self) -> f64 {
        if self.total_tool_calls == 0 {
            0.0
        } else {
            (self.parallel_tool_calls as f64 / self.total_tool_calls as f64) * 100.0
        }
    }

    fn tool_diversity(&self) -> f64 {
        // Shannon entropy: -Σ(p * log2(p))
        let total = self.total_tool_calls as f64;
        if total == 0.0 {
            return 0.0;
        }

        let mut entropy = 0.0;
        for &count in self.tool_name_counts.values() {
            let p = count as f64 / total;
            if p > 0.0 {
                entropy -= p * p.log2();
            }
        }
        entropy
    }

    fn error_rate(&self) -> f64 {
        if self.total_tool_calls == 0 {
            0.0
        } else {
            (self.failed_tool_calls as f64 / self.total_tool_calls as f64) * 100.0
        }
    }

    fn tokens_per_session(&self) -> f64 {
        if self.total_sessions == 0 {
            0.0
        } else {
            self.total_tokens as f64 / self.total_sessions as f64
        }
    }

    fn tokens_per_tool(&self) -> f64 {
        if self.total_tool_calls == 0 {
            0.0
        } else {
            self.total_tokens as f64 / self.total_tool_calls as f64
        }
    }

    fn read_write_execute_ratio(&self) -> (f64, f64, f64) {
        let total = (self.read_calls + self.write_calls + self.execute_calls) as f64;
        if total == 0.0 {
            return (0.0, 0.0, 0.0);
        }
        (
            (self.read_calls as f64 / total) * 100.0,
            (self.write_calls as f64 / total) * 100.0,
            (self.execute_calls as f64 / total) * 100.0,
        )
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== agtrace SDK: Provider Efficiency Analysis ===\n");

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

    // 3. Collect efficiency metrics per provider
    let mut provider_metrics: HashMap<String, ProviderEfficiency> = HashMap::new();

    for session_summary in &sessions {
        let provider = &session_summary.provider;
        let metrics = provider_metrics.entry(provider.clone()).or_default();

        metrics.total_sessions += 1;

        // Analyze session structure
        if let Ok(session_handle) = client.sessions().get(&session_summary.id) {
            if let Ok(session) = session_handle.assemble() {
                // Collect token stats from assembled session
                metrics.total_tokens += session.stats.usage.total_tokens().as_u64() as usize;
                for turn in &session.turns {
                    metrics.total_turns += 1;

                    for step in &turn.steps {
                        metrics.total_steps += 1;

                        let tools_in_step = step.tools.len();

                        // Track parallelization
                        if tools_in_step > 1 {
                            metrics.turns_with_multiple_tools += 1;
                            metrics.parallel_tool_calls += tools_in_step;
                        }

                        for tool_exec in &step.tools {
                            metrics.total_tool_calls += 1;

                            let call = &tool_exec.call.content;

                            // Track tool kinds
                            match call.kind() {
                                ToolKind::Read => metrics.read_calls += 1,
                                ToolKind::Write => metrics.write_calls += 1,
                                ToolKind::Execute => metrics.execute_calls += 1,
                                ToolKind::Search => metrics.search_calls += 1,
                                ToolKind::Plan | ToolKind::Ask | ToolKind::Other => {
                                    metrics.other_calls += 1
                                }
                            }

                            // Track tool names for diversity
                            *metrics
                                .tool_name_counts
                                .entry(call.name().to_string())
                                .or_insert(0) += 1;

                            // Track errors
                            if let Some(result) = &tool_exec.result {
                                if result.content.is_error {
                                    metrics.failed_tool_calls += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 4. Display efficiency comparison
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("PROVIDER EFFICIENCY COMPARISON");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Sort providers by session count
    let mut providers: Vec<_> = provider_metrics.iter().collect();
    providers.sort_by(|a, b| b.1.total_sessions.cmp(&a.1.total_sessions));

    // Display summary table
    println!(
        "{:<15} {:>10} {:>12} {:>14} {:>12} {:>10}",
        "Provider", "Sessions", "Tools/Sess", "Parallel Calls", "Diversity", "Error%"
    );
    println!("{}", "─".repeat(82));

    for (provider_name, metrics) in &providers {
        let warning = if metrics.total_sessions < 20 {
            " ⚠️"
        } else {
            ""
        };
        println!(
            "{:<15} {:>10} {:>12.1} {:>13.1}% {:>12.2} {:>10.2}{}",
            provider_name,
            metrics.total_sessions,
            metrics.tools_per_session(),
            metrics.parallel_calls_ratio(),
            metrics.tool_diversity(),
            metrics.error_rate(),
            warning,
        );
    }

    println!("\n");

    // Display detailed metrics for each provider
    for (provider_name, metrics) in &providers {
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("Provider: {}", provider_name);
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

        // Basic stats
        println!("📊 Basic Statistics:");
        println!("  Sessions:         {}", metrics.total_sessions);
        println!("  Total tool calls: {}", metrics.total_tool_calls);
        println!("  Total turns:      {}", metrics.total_turns);
        println!("  Total steps:      {}", metrics.total_steps);
        println!();

        // Efficiency metrics
        println!("⚡ Efficiency Metrics:");
        println!(
            "  Tools per session:      {:.2}",
            metrics.tools_per_session()
        );
        println!(
            "  Parallelization rate:   {:.2}% (turns with multiple tools)",
            metrics.parallelization_rate()
        );
        println!(
            "  Parallel calls ratio:   {:.2}% (calls executed in parallel)",
            metrics.parallel_calls_ratio()
        );
        println!(
            "  Tool diversity:         {:.2} (Shannon entropy)",
            metrics.tool_diversity()
        );
        println!("  Error rate:             {:.2}%", metrics.error_rate());
        println!();

        // Token efficiency (if available)
        if metrics.total_tokens > 0 {
            println!("🪙 Token Efficiency:");
            println!("  Total tokens:           {}", metrics.total_tokens);
            println!(
                "  Tokens per session:     {:.0}",
                metrics.tokens_per_session()
            );
            println!("  Tokens per tool call:   {:.0}", metrics.tokens_per_tool());

            // Warn if token stats seem abnormal
            let tpt = metrics.tokens_per_tool();
            if tpt > 10000.0 {
                println!(
                    "  ⚠️  Warning: Abnormally high tokens/tool - possible data quality issue"
                );
            }
            println!();
        }

        // Implementation style
        println!("🎨 Implementation Style:");
        let (read_pct, write_pct, exec_pct) = metrics.read_write_execute_ratio();
        println!(
            "  Read:    {:>6} calls ({:>5.1}%)",
            metrics.read_calls, read_pct
        );
        println!(
            "  Write:   {:>6} calls ({:>5.1}%)",
            metrics.write_calls, write_pct
        );
        println!(
            "  Execute: {:>6} calls ({:>5.1}%)",
            metrics.execute_calls, exec_pct
        );
        println!("  Search:  {:>6} calls", metrics.search_calls);
        println!("  Other:   {:>6} calls", metrics.other_calls);
        println!();

        // Top tools
        println!("🔧 Top 5 Tools:");
        let mut tools: Vec<_> = metrics.tool_name_counts.iter().collect();
        tools.sort_by(|a, b| b.1.cmp(a.1));
        for (i, (tool_name, count)) in tools.iter().take(5).enumerate() {
            let pct = (**count as f64 / metrics.total_tool_calls as f64) * 100.0;
            println!("  {}. {:20} × {} ({:.1}%)", i + 1, tool_name, count, pct);
        }
        println!();
    }

    // 5. Comparative analysis
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("COMPARATIVE INSIGHTS");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Filter out providers with insufficient data (< 20 sessions)
    let reliable_providers: Vec<_> = providers
        .iter()
        .filter(|(_, m)| m.total_sessions >= 20)
        .collect();

    if reliable_providers.is_empty() {
        println!(
            "⚠️  Not enough data for reliable comparison (need at least 20 sessions per provider)\n"
        );
        return Ok(());
    }

    // Find best performers among reliable providers
    let best_parallel = reliable_providers
        .iter()
        .max_by(|a, b| {
            a.1.parallel_calls_ratio()
                .partial_cmp(&b.1.parallel_calls_ratio())
                .unwrap()
        })
        .unwrap();

    let best_diversity = reliable_providers
        .iter()
        .max_by(|a, b| {
            a.1.tool_diversity()
                .partial_cmp(&b.1.tool_diversity())
                .unwrap()
        })
        .unwrap();

    let most_efficient = reliable_providers
        .iter()
        .min_by(|a, b| {
            a.1.tools_per_session()
                .partial_cmp(&b.1.tools_per_session())
                .unwrap()
        })
        .unwrap();

    let lowest_error = reliable_providers
        .iter()
        .min_by(|a, b| a.1.error_rate().partial_cmp(&b.1.error_rate()).unwrap())
        .unwrap();

    println!(
        "🏆 Best Parallelization: {} ({:.1}% of calls run in parallel)",
        best_parallel.0,
        best_parallel.1.parallel_calls_ratio()
    );
    println!(
        "🎯 Highest Tool Diversity: {} ({:.2} entropy)",
        best_diversity.0,
        best_diversity.1.tool_diversity()
    );
    println!(
        "⚡ Most Tool Efficient: {} ({:.1} tools/session)",
        most_efficient.0,
        most_efficient.1.tools_per_session()
    );
    println!(
        "✅ Lowest Error Rate: {} ({:.2}%)",
        lowest_error.0,
        lowest_error.1.error_rate()
    );

    if reliable_providers
        .iter()
        .any(|(_, m)| m.total_tokens > 0 && m.total_sessions > 0)
    {
        let best_token_efficiency = reliable_providers
            .iter()
            .filter(|(_, m)| m.total_sessions > 0)
            .min_by(|a, b| {
                a.1.tokens_per_tool()
                    .partial_cmp(&b.1.tokens_per_tool())
                    .unwrap()
            })
            .unwrap();

        println!(
            "💰 Best Token Efficiency: {} ({:.0} tokens/tool)",
            best_token_efficiency.0,
            best_token_efficiency.1.tokens_per_tool()
        );
    }

    println!();

    Ok(())
}
