//! Lightweight provider operations without database
//!
//! This example demonstrates using `Providers` for operations
//! that don't require a full workspace with database:
//! - Auto-detecting providers
//! - Running diagnostics
//! - Listing provider configuration
//!
//! Use this approach when you only need read-only file access
//! and don't need session querying or indexing.
//!
//! Run with: cargo run -p agtrace-sdk --example providers_only

use agtrace_sdk::Providers;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Auto-detect providers from system paths (no database needed)
    let providers = Providers::detect()?;

    println!("Detected providers:");
    for (name, config) in providers.list() {
        println!("  {}: {:?}", name, config.log_root);
    }

    // Run diagnostics on all providers
    println!("\nRunning diagnostics...");
    let results = providers.diagnose()?;

    for result in &results {
        let success_rate = if result.total_files > 0 {
            (result.successful as f64 / result.total_files as f64) * 100.0
        } else {
            100.0
        };
        println!(
            "  {}: {:.1}% success ({}/{} files)",
            result.provider_name, success_rate, result.successful, result.total_files
        );

        // Show failure categories if any
        for (failure_type, examples) in &result.failures {
            println!("    {:?}: {} files", failure_type, examples.len());
        }
    }

    // Example: Custom provider configuration
    println!("\nCustom configuration example:");
    let _custom = Providers::builder()
        .provider("claude_code", "/custom/path/.claude/projects")
        .build()?;
    println!("  Created Providers with custom path");

    Ok(())
}
