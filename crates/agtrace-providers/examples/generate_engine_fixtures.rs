// Example: Generate AgentEvent fixtures for engine tests
//
// This demonstrates how to use the provider normalization functions to convert
// raw provider data into standardized AgentEvent arrays.
//
// Usage:
//   cargo run --package agtrace-providers --example generate_engine_fixtures

use std::fs;
use std::path::Path;

fn main() -> anyhow::Result<()> {
    let output_dir = Path::new("crates/agtrace-engine/tests/fixtures");
    fs::create_dir_all(output_dir)?;

    println!("Generating engine test fixtures from provider samples...\n");

    // Claude session
    let claude_path = Path::new("crates/agtrace-providers/tests/samples/claude_session.jsonl");
    if claude_path.exists() {
        let events = agtrace_providers::normalize_claude_file(claude_path)?;
        let json = serde_json::to_string_pretty(&events)?;
        fs::write(output_dir.join("claude_events.json"), json)?;
        println!("✓ Generated claude_events.json ({} events)", events.len());
    }

    // Codex session
    let codex_path = Path::new("crates/agtrace-providers/tests/samples/codex_session.jsonl");
    if codex_path.exists() {
        let events = agtrace_providers::normalize_codex_file(codex_path)?;
        let json = serde_json::to_string_pretty(&events)?;
        fs::write(output_dir.join("codex_events.json"), json)?;
        println!("✓ Generated codex_events.json ({} events)", events.len());
    }

    // Gemini session
    let gemini_path = Path::new("crates/agtrace-providers/tests/samples/gemini_session.json");
    if gemini_path.exists() {
        let events = agtrace_providers::normalize_gemini_file(gemini_path)?;
        let json = serde_json::to_string_pretty(&events)?;
        fs::write(output_dir.join("gemini_events.json"), json)?;
        println!("✓ Generated gemini_events.json ({} events)", events.len());
    }

    println!("\nAll fixtures generated in {}", output_dir.display());
    Ok(())
}
