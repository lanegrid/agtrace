use crate::cli::import::{count_unique_sessions, import_vendor_logs};
use crate::utils::discover_project_root;
use anyhow::Result;

pub fn handle(project_root: Option<String>) -> Result<()> {
    let project_root_path = discover_project_root(project_root.as_deref())?;
    let target_hash = crate::utils::project_hash_from_root(&project_root_path.to_string_lossy());

    println!("Project root: {}", project_root_path.display());
    println!("Project hash: {}", target_hash);
    println!();

    let config = crate::config::Config::load()?;
    println!("Providers:");

    for (name, provider_config) in &config.providers {
        if !provider_config.enabled {
            continue;
        }

        println!("  {}:", name);
        println!("    log_root: {}", provider_config.log_root.display());

        let events_result = import_vendor_logs(name, None, None, None, true);

        match events_result {
            Ok(events) => {
                let total_sessions = count_unique_sessions(&events);

                let matching_sessions = events
                    .iter()
                    .filter(|e| e.project_hash == target_hash)
                    .filter_map(|e| e.session_id.as_ref())
                    .collect::<std::collections::HashSet<_>>()
                    .len();

                println!("    sessions detected: {}", total_sessions);
                println!("    sessions matching this project: {}", matching_sessions);
            }
            Err(e) => {
                println!("    Error scanning logs: {}", e);
            }
        }
        println!();
    }

    Ok(())
}
