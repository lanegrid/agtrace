use super::args::{
    Cli, Commands, DoctorCommand, IndexCommand, LabCommand, ProjectCommand, ProviderCommand,
    SessionCommand,
};
use super::handlers;
use crate::config::Config;
use agtrace_index::Database;
use anyhow::Result;
use std::path::PathBuf;

fn print_deprecation_warning(old_cmd: &str, new_cmd: &str, format: &str) {
    if std::env::var("AGTRACE_NO_DEPRECATION_WARN").is_ok() {
        return;
    }

    if format == "json" {
        return;
    }

    eprintln!(
        "Warning: '{}' is deprecated; use '{}' instead",
        old_cmd, new_cmd
    );
}

pub fn run(cli: Cli) -> Result<()> {
    let data_dir = expand_tilde(&cli.data_dir);

    let Some(command) = cli.command else {
        show_guidance(&data_dir)?;
        return Ok(());
    };

    match command {
        Commands::Init => handlers::init::handle(&data_dir, cli.project_root, cli.all_projects),

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
                ProviderCommand::List => handlers::providers::handle(None, &config_path),
                ProviderCommand::Detect => handlers::providers::handle(
                    Some(super::args::ProvidersCommand::Detect),
                    &config_path,
                ),
                ProviderCommand::Set {
                    provider,
                    log_root,
                    enable,
                    disable,
                } => handlers::providers::handle(
                    Some(super::args::ProvidersCommand::Set {
                        provider,
                        log_root,
                        enable,
                        disable,
                    }),
                    &config_path,
                ),
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

        // Legacy commands (with deprecation warnings)
        Commands::Scan {
            provider,
            force,
            verbose,
        } => {
            if force {
                print_deprecation_warning("scan --force", "index rebuild", &cli.format);
            } else {
                print_deprecation_warning("scan", "index update", &cli.format);
            }

            let db_path = data_dir.join("agtrace.db");
            let db = Database::open(&db_path)?;
            let config_path = data_dir.join("config.toml");
            let config = Config::load_from(&config_path)?;

            handlers::scan::handle(
                &db,
                &config,
                provider,
                cli.project_root,
                cli.all_projects,
                force,
                verbose,
            )
        }

        Commands::List {
            project_hash,
            source: _,
            limit,
            since: _,
            until: _,
        } => {
            print_deprecation_warning("list", "session list", &cli.format);

            let db_path = data_dir.join("agtrace.db");
            let db = Database::open(&db_path)?;

            let effective_hash = if project_hash.is_none() && cli.project_root.is_some() {
                Some(agtrace_types::project_hash_from_root(
                    cli.project_root.as_ref().unwrap(),
                ))
            } else {
                project_hash
            };

            handlers::list::handle(&db, effective_hash, limit, cli.all_projects, &cli.format)
        }

        Commands::View {
            session_id,
            raw,
            json,
            timeline,
            hide,
            only,
            full,
            short,
            style,
        } => {
            print_deprecation_warning("view", "session show", &cli.format);

            let db_path = data_dir.join("agtrace.db");
            let db = Database::open(&db_path)?;

            handlers::view::handle(
                &db, session_id, raw, json, timeline, hide, only, full, short, style,
            )
        }

        Commands::Show { .. } => {
            anyhow::bail!("'show' command has been removed. Use 'session show <session-id>' instead.")
        }

        Commands::Find { .. } => {
            anyhow::bail!("'find' command has been removed. Search functionality will be available in future releases.")
        }

        Commands::Stats { .. } => {
            anyhow::bail!("'stats' command has been removed. Statistics functionality will be available in future releases.")
        }

        Commands::Export {
            session_id,
            output,
            format,
            strategy,
        } => {
            print_deprecation_warning("export", "lab export", &cli.format);

            let db_path = data_dir.join("agtrace.db");
            let db = Database::open(&db_path)?;
            handlers::export::handle(&db, session_id, output, format, strategy)
        }

        Commands::Analyze {
            session_id,
            detect,
            format,
        } => {
            print_deprecation_warning("analyze", "lab analyze", &cli.format);

            let db_path = data_dir.join("agtrace.db");
            let db = Database::open(&db_path)?;
            handlers::analyze::handle(&db, session_id, detect, format)
        }

        Commands::Providers { command } => {
            print_deprecation_warning("providers", "provider", &cli.format);

            let config_path = data_dir.join("config.toml");
            handlers::providers::handle(command, &config_path)
        }

        Commands::Diagnose { provider, verbose } => {
            print_deprecation_warning("diagnose", "doctor run", &cli.format);

            let config_path = data_dir.join("config.toml");
            let config = Config::load_from(&config_path)?;
            handlers::diagnose::handle(&config, provider, verbose)
        }

        Commands::Inspect {
            file_path,
            lines,
            format,
        } => {
            print_deprecation_warning("inspect", "doctor inspect", &cli.format);

            handlers::inspect::handle(file_path, lines, format)
        }

        Commands::Validate {
            file_path,
            provider,
        } => {
            print_deprecation_warning("validate", "doctor check", &cli.format);

            handlers::validate::handle(file_path, provider)
        }

        Commands::Schema { provider, format } => {
            print_deprecation_warning("schema", "provider schema", &cli.format);

            handlers::schema::handle(provider, format)
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
        println!("Quick commands:");
        println!("  agtrace session list              # View recent sessions");
        println!("  agtrace index update              # Scan for new sessions");
        println!("  agtrace session show <ID>         # View a session");
        println!("  agtrace doctor run                # Diagnose issues\n");
    }

    println!("For more commands:");
    println!("  agtrace --help");

    Ok(())
}
