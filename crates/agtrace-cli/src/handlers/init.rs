use crate::config::Config;
use agtrace_index::Database;
use anyhow::Result;
use std::path::PathBuf;

pub fn handle(
    data_dir: &PathBuf,
    project_root: Option<String>,
    all_projects: bool,
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

    println!("\nStep 3/4: Scanning for sessions...");
    let scan_result = super::scan::handle(
        &db,
        &config,
        "all".to_string(),
        project_root,
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

    println!("\nStep 4/4: Recent sessions...\n");

    let sessions = db.list_sessions(None, 10)?;

    if sessions.is_empty() {
        println!("No sessions found in the current project.");
        println!("\nTips:");
        println!("  - To scan all projects: agtrace index update --all-projects");
        println!("  - To check a specific project: agtrace index update --project-root <PATH>");
        return Ok(());
    }

    super::list::handle(&db, None, 10, all_projects, "plain")?;

    if let Some(first_session) = sessions.first() {
        let session_prefix = if first_session.id.len() > 8 {
            &first_session.id[..8]
        } else {
            &first_session.id
        };

        println!("\nNext steps:");
        println!("  View session in compact style:");
        println!("    agtrace session show {} --style compact", session_prefix);
        println!("\n  View conversation only:");
        println!(
            "    agtrace session show {} --only user,assistant --full",
            session_prefix
        );
    }

    Ok(())
}
