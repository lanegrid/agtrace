// Example: Generate AgentEvent fixtures for engine tests
//
// This demonstrates how to use the trait-based ProviderAdapter to convert
// raw provider data into standardized AgentEvent arrays.
//
// Usage:
//   cargo run --package agtrace-providers --example generate_engine_fixtures

use agtrace_providers::ProviderAdapter;
use std::fs;
use std::path::Path;

struct ProviderConfig {
    name: &'static str,
    sample_path: &'static str,
    output_file: &'static str,
}

fn main() -> anyhow::Result<()> {
    let output_dir = Path::new("crates/agtrace-engine/tests/fixtures");
    fs::create_dir_all(output_dir)?;

    println!("Generating engine test fixtures from provider samples...\n");

    let configs = [
        ProviderConfig {
            name: "claude",
            sample_path: "crates/agtrace-providers/tests/samples/claude_session.jsonl",
            output_file: "claude_events.json",
        },
        ProviderConfig {
            name: "codex",
            sample_path: "crates/agtrace-providers/tests/samples/codex_session.jsonl",
            output_file: "codex_events.json",
        },
        ProviderConfig {
            name: "gemini",
            sample_path: "crates/agtrace-providers/tests/samples/gemini_session.json",
            output_file: "gemini_events.json",
        },
    ];

    for config in &configs {
        let sample_path = Path::new(config.sample_path);
        if !sample_path.exists() {
            continue;
        }

        let adapter = ProviderAdapter::from_name(config.name)?;
        // Use parser directly instead of process_file to skip probe checks
        // (sample files may not match production naming conventions)
        let events = adapter.parser.parse_file(sample_path)?;
        let json = serde_json::to_string_pretty(&events)?;
        fs::write(output_dir.join(config.output_file), json)?;
        println!(
            "✓ Generated {} ({} events)",
            config.output_file,
            events.len()
        );
    }

    println!("\nAll fixtures generated in {}", output_dir.display());
    Ok(())
}
