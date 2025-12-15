use super::args::{
    Cli, Commands, DoctorCommand, IndexCommand, LabCommand, ProjectCommand, ProviderCommand,
    SessionCommand,
};
use super::handlers;
use crate::config::Config;
use agtrace_index::Database;
use anyhow::Result;
use std::path::{Path, PathBuf};

pub fn run(cli: Cli) -> Result<()> {
    let data_dir = expand_tilde(&cli.data_dir);

    let Some(command) = cli.command else {
        // Check if we should show corpus overview or guidance
        let db_path = data_dir.join("agtrace.db");
        if db_path.exists() {
            if let Ok(db) = Database::open(&db_path) {
                if let Ok(sessions) = db.list_sessions(None, 1) {
                    if !sessions.is_empty() {
                        // Show corpus overview instead of guidance
                        return handlers::corpus_overview::handle(
                            &db,
                            cli.project_root,
                            cli.all_projects,
                        );
                    }
                }
            }
        }
        show_guidance(&data_dir)?;
        return Ok(());
    };

    match command {
        Commands::Init { refresh } => {
            handlers::init::handle(&data_dir, cli.project_root, cli.all_projects, refresh)
        }

        Commands::Index { command } => {
            let db_path = data_dir.join("agtrace.db");
            let db = Database::open(&db_path)?;
            let config_path = data_dir.join("config.toml");
            let config = Config::load_from(&config_path)?;

            match command {
                IndexCommand::Update { provider, verbose } => handlers::index::handle(
                    &db,
                    &config,
                    provider,
                    cli.project_root,
                    cli.all_projects,
                    false,
                    verbose,
                ),
                IndexCommand::Rebuild { provider, verbose } => handlers::index::handle(
                    &db,
                    &config,
                    provider,
                    cli.project_root,
                    cli.all_projects,
                    true,
                    verbose,
                ),
                IndexCommand::Vacuum => {
                    let db_path = data_dir.join("agtrace.db");
                    let db = Database::open(&db_path)?;
                    db.vacuum()
                }
            }
        }

        Commands::Session { command } => {
            let db_path = data_dir.join("agtrace.db");
            let db = Database::open(&db_path)?;

            match command {
                SessionCommand::List {
                    project_hash,
                    source,
                    limit,
                    since,
                    until,
                } => {
                    let effective_hash = if project_hash.is_none() && cli.project_root.is_some() {
                        Some(agtrace_types::project_hash_from_root(
                            cli.project_root.as_ref().unwrap(),
                        ))
                    } else {
                        project_hash
                    };

                    handlers::session_list::handle(
                        &db,
                        effective_hash,
                        limit,
                        cli.all_projects,
                        &cli.format,
                        source.clone(),
                        since.clone(),
                        until.clone(),
                    )
                }
                SessionCommand::Show {
                    session_id,
                    raw,
                    json,
                    timeline,
                    hide,
                    only,
                    full,
                    short,
                    style,
                } => handlers::session_show::handle(
                    &db, session_id, raw, json, timeline, hide, only, full, short, style,
                ),
            }
        }

        Commands::Provider { command } => {
            let config_path = data_dir.join("config.toml");

            match command {
                ProviderCommand::List => handlers::provider::list(&config_path),
                ProviderCommand::Detect => handlers::provider::detect(&config_path),
                ProviderCommand::Set {
                    provider,
                    log_root,
                    enable,
                    disable,
                } => handlers::provider::set(provider, log_root, enable, disable, &config_path),
                ProviderCommand::Schema { provider, format } => {
                    handlers::provider_schema::handle(provider, format)
                }
            }
        }

        Commands::Doctor { command } => match command {
            DoctorCommand::Run { provider, verbose } => {
                let config_path = data_dir.join("config.toml");
                let config = Config::load_from(&config_path)?;
                handlers::doctor_run::handle(&config, provider, verbose)
            }
            DoctorCommand::Inspect {
                file_path,
                lines,
                format,
            } => handlers::doctor_inspect::handle(file_path, lines, format),
            DoctorCommand::Check {
                file_path,
                provider,
            } => handlers::doctor_check::handle(file_path, provider),
        },

        Commands::Project { command } => {
            let db_path = data_dir.join("agtrace.db");
            let db = Database::open(&db_path)?;

            match command {
                ProjectCommand::List { project_root } => {
                    handlers::project::handle(&db, project_root)
                }
            }
        }

        Commands::Lab { command } => {
            let db_path = data_dir.join("agtrace.db");
            let db = Database::open(&db_path)?;

            match command {
                LabCommand::Export {
                    session_id,
                    output,
                    format,
                    strategy,
                } => handlers::lab_export::handle(&db, session_id, output, format, strategy),
            }
        }

        Commands::Pack { template, limit } => {
            let db_path = data_dir.join("agtrace.db");
            let db = Database::open(&db_path)?;

            handlers::pack::handle(&db, &template, limit, cli.project_root, cli.all_projects)
        }

        Commands::Watch { provider, id } => {
            let config_path = data_dir.join("config.toml");
            let config = Config::load_from(&config_path)?;

            // Determine which provider to watch
            let provider_name = provider.unwrap_or_else(|| {
                // Auto-detect: use first enabled provider
                config
                    .providers
                    .iter()
                    .find(|(_, p)| p.enabled)
                    .map(|(name, _)| name.clone())
                    .unwrap_or_else(|| "claude_code".to_string())
            });

            let provider_config = config.providers.get(&provider_name).ok_or_else(|| {
                anyhow::anyhow!("Provider '{}' not found in config", provider_name)
            })?;

            if !provider_config.enabled {
                anyhow::bail!("Provider '{}' is not enabled. Run 'agtrace provider list' to see available providers.", provider_name);
            }

            handlers::watch::handle(&provider_config.log_root, id)
        }
    }
}

fn expand_tilde(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(stripped);
        }
    }
    PathBuf::from(path)
}

fn show_guidance(data_dir: &Path) -> Result<()> {
    let config_path = data_dir.join("config.toml");
    let db_path = data_dir.join("agtrace.db");

    let config_exists = config_path.exists();
    let db_exists = db_path.exists();

    println!("agtrace - Agent behavior log analyzer\n");

    if !config_exists || !db_exists {
        println!("Get started:");
        println!("  agtrace init\n");
        println!("The init command will:");
        println!("  1. Detect and configure providers (Claude, Codex, Gemini)");
        println!("  2. Set up the database");
        println!("  3. Scan for sessions");
        println!("  4. Show your recent sessions\n");
    } else {
        let db = Database::open(&db_path)?;
        let session_count = db.list_sessions(None, 1)?.len();

        if session_count > 0 {
            println!("Quick commands:");
            println!("  agtrace session list              # View recent sessions");
            println!("  agtrace session show <ID>         # View a session");
            println!("  agtrace index update              # Scan for new sessions");
            println!("  agtrace doctor run                # Diagnose issues\n");
        } else {
            println!("No sessions found yet.");
            println!("\nNext steps:");
            println!("  agtrace index update              # Scan for sessions");
            println!("  agtrace index update --all-projects  # Scan all projects");
            println!("  agtrace provider list             # Check provider configuration\n");
        }
    }

    println!("For more commands:");
    println!("  agtrace --help");

    Ok(())
}
