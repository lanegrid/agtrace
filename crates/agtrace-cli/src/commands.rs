use super::args::{
    Cli, Commands, DoctorCommand, IndexCommand, LabCommand, ProjectCommand, ProviderCommand,
    SessionCommand,
};
use super::handlers;
use crate::config::Config;
use agtrace_index::Database;
use anyhow::Result;
use std::path::PathBuf;

pub fn run(cli: Cli) -> Result<()> {
    let data_dir = expand_tilde(&cli.data_dir);

    let Some(command) = cli.command else {
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
                IndexCommand::Update { provider, verbose } => handlers::scan::handle(
                    &db,
                    &config,
                    provider,
                    cli.project_root,
                    cli.all_projects,
                    false,
                    verbose,
                ),
                IndexCommand::Rebuild { provider, verbose } => handlers::scan::handle(
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
                    source: _,
                    limit,
                    since: _,
                    until: _,
                } => {
                    let effective_hash = if project_hash.is_none() && cli.project_root.is_some() {
                        Some(agtrace_types::project_hash_from_root(
                            cli.project_root.as_ref().unwrap(),
                        ))
                    } else {
                        project_hash
                    };

                    handlers::list::handle(
                        &db,
                        effective_hash,
                        limit,
                        cli.all_projects,
                        &cli.format,
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
                } => handlers::view::handle(
                    &db, session_id, raw, json, timeline, hide, only, full, short, style,
                ),
            }
        }

        Commands::Provider { command } => {
            let config_path = data_dir.join("config.toml");

            match command {
                ProviderCommand::List => handlers::providers::list(&config_path),
                ProviderCommand::Detect => handlers::providers::detect(&config_path),
                ProviderCommand::Set {
                    provider,
                    log_root,
                    enable,
                    disable,
                } => handlers::providers::set(provider, log_root, enable, disable, &config_path),
                ProviderCommand::Schema { provider, format } => {
                    handlers::schema::handle(provider, format)
                }
            }
        }

        Commands::Doctor { command } => match command {
            DoctorCommand::Run { provider, verbose } => {
                let config_path = data_dir.join("config.toml");
                let config = Config::load_from(&config_path)?;
                handlers::diagnose::handle(&config, provider, verbose)
            }
            DoctorCommand::Inspect {
                file_path,
                lines,
                format,
            } => handlers::inspect::handle(file_path, lines, format),
            DoctorCommand::Check {
                file_path,
                provider,
            } => handlers::validate::handle(file_path, provider),
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
                LabCommand::Analyze {
                    session_id,
                    detect,
                    format,
                } => handlers::analyze::handle(&db, session_id, detect, format),
                LabCommand::Export {
                    session_id,
                    output,
                    format,
                    strategy,
                } => handlers::export::handle(&db, session_id, output, format, strategy),
            }
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

fn show_guidance(data_dir: &PathBuf) -> Result<()> {
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
