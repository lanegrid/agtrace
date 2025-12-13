use crate::config::Config;
use agtrace_index::Database;
use agtrace_types::project_hash_from_root;
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use std::path::Path;

fn format_duration(d: Duration) -> String {
    let seconds = d.num_seconds();
    let minutes = d.num_minutes();
    if seconds < 60 {
        format!("{}s ago", seconds)
    } else if minutes < 60 {
        format!("{}m ago", minutes)
    } else {
        let hours = d.num_hours();
        format!("{}h ago", hours)
    }
}

pub fn handle(
    data_dir: &Path,
    project_root: Option<String>,
    all_projects: bool,
    refresh: bool,
) -> Result<()> {
    println!("Initializing agtrace...\n");

    let config_path = data_dir.join("config.toml");
    let db_path = data_dir.join("agtrace.db");

    let config = if !config_path.exists() {
        println!("Step 1/4: Detecting providers...");
        let detected = Config::detect_providers()?;

        if detected.providers.is_empty() {
            println!("  No providers detected automatically.");
            println!("\n  To manually configure a provider:");
            println!("    agtrace provider set <name> --log-root <PATH> --enable");
            println!("\n  Supported providers:");
            println!("    - claude  (default: ~/.claude/projects)");
            println!("    - codex   (default: ~/.codex/sessions)");
            println!("    - gemini  (default: ~/.gemini/tmp)");
            return Ok(());
        }

        println!("  Detected {} provider(s):", detected.providers.len());
        for (name, provider_config) in &detected.providers {
            println!("    {} -> {}", name, provider_config.log_root.display());
        }

        detected.save_to(&config_path)?;
        println!("  Configuration saved to {}", config_path.display());

        detected
    } else {
        println!("Step 1/4: Loading configuration...");
        let cfg = Config::load_from(&config_path)?;
        println!("  Configuration loaded from {}", config_path.display());
        cfg
    };

    println!("\nStep 2/4: Setting up database...");
    let db = Database::open(&db_path)?;
    println!("  Database ready at {}", db_path.display());

    let current_project_root = if let Some(root) = &project_root {
        root.clone()
    } else if let Ok(cwd) = std::env::current_dir() {
        cwd.display().to_string()
    } else {
        ".".to_string()
    };
    let current_project_hash = project_hash_from_root(&current_project_root);

    let should_scan = if refresh {
        true
    } else if let Ok(Some(project)) = db.get_project(&current_project_hash) {
        if let Some(last_scanned) = &project.last_scanned_at {
            if let Ok(last_time) = DateTime::parse_from_rfc3339(last_scanned) {
                let elapsed = Utc::now().signed_duration_since(last_time.with_timezone(&Utc));
                if elapsed < Duration::minutes(5) {
                    println!("\nStep 3/4: Scanning for sessions...");
                    println!(
                        "  Recently scanned ({}). Skipping.",
                        format_duration(elapsed)
                    );
                    println!("  Use `agtrace init --refresh` to force re-scan.");
                    false
                } else {
                    true
                }
            } else {
                true
            }
        } else {
            true
        }
    } else {
        true
    };

    if should_scan {
        println!("\nStep 3/4: Scanning for sessions...");
        let scan_result = super::index::handle(
            &db,
            &config,
            "all".to_string(),
            Some(current_project_root.clone()),
            all_projects,
            false,
            false,
        );

        match scan_result {
            Ok(_) => {}
            Err(e) => {
                println!("  Warning: Scan completed with errors: {}", e);
                println!("\n  If you encounter compatibility issues, run:");
                println!("    agtrace doctor run");
            }
        }
    }

    println!("\nStep 4/4: Recent sessions...\n");

    let effective_hash = if all_projects {
        None
    } else {
        Some(current_project_hash.clone())
    };

    let sessions = db.list_sessions(effective_hash.as_deref(), 10)?;

    if sessions.is_empty() {
        if all_projects {
            println!("No sessions found.");
            println!("\nTips:");
            println!("  - Check provider configuration: agtrace provider list");
            println!("  - Run diagnostics: agtrace doctor run");
        } else {
            println!("No sessions found for the current project.");
            println!("\nTips:");
            println!("  - Scan all projects: agtrace init --all-projects");
            println!("  - Or: agtrace index update --all-projects");
        }
        return Ok(());
    }

    super::session_list::handle(
        &db,
        effective_hash,
        10,
        all_projects,
        "plain",
        None,
        None,
        None,
    )?;

    if let Some(first_session) = sessions.first() {
        let session_prefix = if first_session.id.len() > 8 {
            &first_session.id[..8]
        } else {
            &first_session.id
        };

        println!("\nNext steps:");
        println!("  View session in compact style (see bottlenecks and tool chains):");
        println!(
            "    agtrace session show {} --style compact",
            session_prefix
        );
        println!("\n  View conversation only (for LLM consumption):");
        println!(
            "    agtrace session show {} --only user,assistant --full",
            session_prefix
        );
    }

    Ok(())
}
