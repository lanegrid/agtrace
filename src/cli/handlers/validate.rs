use crate::providers::{ClaudeProvider, CodexProvider, GeminiProvider, ImportContext, LogProvider};
use anyhow::Result;
use owo_colors::OwoColorize;
use std::path::Path;

pub fn handle(file_path: String, provider_override: Option<String>) -> Result<()> {
    let path = Path::new(&file_path);

    if !path.exists() {
        anyhow::bail!("File not found: {}", file_path);
    }

    // Auto-detect or use specified provider
    let (provider, provider_name): (Box<dyn LogProvider>, String) = if let Some(name) = provider_override {
        match name.as_str() {
            "claude" => (Box::new(ClaudeProvider::new()), "claude".to_string()),
            "codex" => (Box::new(CodexProvider::new()), "codex".to_string()),
            "gemini" => (Box::new(GeminiProvider::new()), "gemini".to_string()),
            _ => anyhow::bail!("Unknown provider: {}", name),
        }
    } else {
        // Auto-detect from path
        if file_path.contains(".claude/") {
            (Box::new(ClaudeProvider::new()), "claude (auto-detected)".to_string())
        } else if file_path.contains(".codex/") {
            (Box::new(CodexProvider::new()), "codex (auto-detected)".to_string())
        } else if file_path.contains(".gemini/") {
            (Box::new(GeminiProvider::new()), "gemini (auto-detected)".to_string())
        } else {
            anyhow::bail!("Cannot auto-detect provider from path. Use --provider to specify.");
        }
    };

    println!("File: {}", file_path);
    println!("Provider: {}", provider_name);

    let context = ImportContext {
        project_root_override: None,
        session_id_prefix: None,
        all_projects: true,
    };

    match provider.normalize_file(path, &context) {
        Ok(events) => {
            println!("Status: {}", "✓ Valid".green().bold());
            println!();
            println!("Parsed successfully:");

            // Extract session info
            if let Some(first_event) = events.first() {
                if let Some(session_id) = &first_event.session_id {
                    println!("  - Session ID: {}", session_id);
                }
                if let Some(project_root) = &first_event.project_root {
                    println!("  - Project: {}", project_root);
                }
            }

            println!("  - Events extracted: {}", events.len());

            // Show event type breakdown
            let mut type_counts = std::collections::HashMap::new();
            for event in &events {
                *type_counts.entry(format!("{:?}", event.event_type)).or_insert(0) += 1;
            }

            if !type_counts.is_empty() {
                println!("  - Event breakdown:");
                for (event_type, count) in type_counts {
                    println!("      {}: {}", event_type, count);
                }
            }
        }
        Err(e) => {
            println!("Status: {}", "✗ Invalid".red().bold());
            println!();
            println!("Parse error:");
            println!("  {}", format!("{:#}", e).red());
            println!();

            // Try to provide helpful suggestions
            let error_msg = format!("{:#}", e);

            if error_msg.contains("missing field") {
                println!("{}", "Suggestion:".cyan().bold());
                println!("  This field may have been added in a newer version of the provider.");
                println!("  Check if the schema definition needs to make this field optional.");
            } else if error_msg.contains("invalid type") {
                println!("{}", "Suggestion:".cyan().bold());
                println!("  The field type in the schema may not match the actual data format.");
                println!("  Use 'agtrace inspect {}' to examine the actual structure.", file_path);
                println!("  Use 'agtrace schema <provider>' to see the expected format.");
            } else if error_msg.contains("expected") {
                println!("{}", "Suggestion:".cyan().bold());
                println!("  The data format may have changed between provider versions.");
                println!("  Consider using an enum or untagged union to support multiple formats.");
            }

            println!();
            println!("Next steps:");
            println!("  1. Examine the actual data:");
            println!("       agtrace inspect {} --lines 20", file_path);
            println!("  2. Compare with expected schema:");
            println!("       agtrace schema {}", provider_name.split_whitespace().next().unwrap_or(""));
            println!("  3. Update schema definition if needed");

            anyhow::bail!("Validation failed");
        }
    }

    Ok(())
}
