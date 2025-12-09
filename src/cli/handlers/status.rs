use crate::cli::import::{count_claude_sessions, count_codex_sessions, count_gemini_sessions};
use crate::utils::discover_project_root;
use anyhow::Result;

pub fn handle(project_root: Option<String>) -> Result<()> {
    let project_root_path = discover_project_root(project_root.as_deref())?;
    let project_hash =
        crate::utils::project_hash_from_root(&project_root_path.to_string_lossy());

    println!("Project root: {}", project_root_path.display());
    println!("Project hash: {}", project_hash);
    println!();

    let config = crate::config::Config::load()?;
    println!("Providers:");

    for (name, provider_config) in &config.providers {
        if !provider_config.enabled {
            continue;
        }

        println!("  {}:", name);
        println!("    log_root: {}", provider_config.log_root.display());

        let (total, matching) = match name.as_str() {
            "claude" => {
                count_claude_sessions(&provider_config.log_root, &project_root_path)
            }
            "codex" => count_codex_sessions(&provider_config.log_root, &project_root_path),
            "gemini" => count_gemini_sessions(&provider_config.log_root, &project_hash),
            _ => (0, 0),
        };

        println!("    sessions detected: {}", total);
        println!("    sessions matching this project: {}", matching);
        println!();
    }

    Ok(())
}
