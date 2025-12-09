use crate::core::importer::ImportService;
use crate::model::*;
use crate::providers::{ClaudeProvider, CodexProvider, GeminiProvider, ImportContext, LogProvider};
use anyhow::Result;
use std::path::PathBuf;

pub fn import_vendor_logs(
    source: &str,
    root: Option<&PathBuf>,
    project_root_override: Option<&str>,
    session_id_prefix: Option<&str>,
    all_projects: bool,
) -> Result<Vec<AgentEventV1>> {
    let context = ImportContext {
        project_root_override: project_root_override.map(|s| s.to_string()),
        session_id_prefix: session_id_prefix.map(|s| s.to_string()),
        all_projects,
    };

    if source == "all" {
        if root.is_some() {
            anyhow::bail!("Cannot specify --root when using --source=all");
        }

        let config = crate::config::Config::load()?;
        let enabled_providers = config.enabled_providers();

        if enabled_providers.is_empty() {
            eprintln!("Warning: No enabled providers found in config. Run 'agtrace providers detect' first.");
            return Ok(Vec::new());
        }

        let mut all_events = Vec::new();

        for (provider_name, provider_config) in enabled_providers {
            println!(
                "Importing from {} (log_root: {})",
                provider_name,
                provider_config.log_root.display()
            );

            let provider: Box<dyn LogProvider> = match provider_name.as_str() {
                "claude" => Box::new(ClaudeProvider::new()),
                "codex" => Box::new(CodexProvider::new()),
                "gemini" => Box::new(GeminiProvider::new()),
                _ => {
                    eprintln!("Warning: Unknown provider '{}', skipping", provider_name);
                    continue;
                }
            };

            let service = ImportService::new(vec![provider]);
            let events = service.import_path(&provider_config.log_root, &context)?;
            all_events.extend(events);
        }

        return Ok(all_events);
    }

    let root_path = if let Some(r) = root {
        r.clone()
    } else {
        let config = crate::config::Config::load()?;
        if let Some(provider_config) = config.providers.get(source) {
            provider_config.log_root.clone()
        } else {
            anyhow::bail!(
                "Provider '{}' not found in config. Run 'agtrace providers detect' first.",
                source
            );
        }
    };

    let provider: Box<dyn LogProvider> = match source {
        "claude" => Box::new(ClaudeProvider::new()),
        "codex" => Box::new(CodexProvider::new()),
        "gemini" => Box::new(GeminiProvider::new()),
        _ => anyhow::bail!("Unknown source: {}", source),
    };

    let service = ImportService::new(vec![provider]);
    service.import_path(&root_path, &context)
}

pub fn count_unique_sessions(events: &[AgentEventV1]) -> usize {
    let mut sessions = std::collections::HashSet::new();
    for event in events {
        if let Some(sid) = &event.session_id {
            sessions.insert(sid.clone());
        }
    }
    sessions.len()
}
