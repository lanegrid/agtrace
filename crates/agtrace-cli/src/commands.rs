use super::args::{
    Cli, Commands, DoctorCommand, IndexCommand, LabCommand, ProjectCommand, ProviderCommand,
    SessionCommand,
};
use super::handlers;
use agtrace_runtime::AgTrace;
use anyhow::Result;
use clap::CommandFactory;
use is_terminal::IsTerminal;
use std::path::PathBuf;

pub fn run(cli: Cli) -> Result<()> {
    let data_dir = expand_tilde(&cli.data_dir);

    let Some(command) = cli.command else {
        Cli::command().print_help()?;
        return Ok(());
    };

    let project_root = cli
        .project_root
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok());

    match command {
        Commands::Init { refresh } => {
            let default_view_mode = crate::args::ViewModeArgs {
                quiet: false,
                compact: false,
                verbose: false,
            };
            handlers::init::handle(
                &data_dir,
                project_root.clone(),
                cli.all_projects,
                refresh,
                cli.format,
                &default_view_mode,
            )
        }

        Commands::Index { command } => {
            let workspace = AgTrace::open(data_dir.clone())?;

            match command {
                IndexCommand::Update {
                    provider,
                    view_mode,
                } => handlers::index::handle(
                    &workspace,
                    project_root.as_deref(),
                    cli.all_projects,
                    provider.to_string(),
                    false,
                    view_mode.verbose,
                    cli.format,
                    &view_mode,
                ),
                IndexCommand::Rebuild {
                    provider,
                    view_mode,
                } => handlers::index::handle(
                    &workspace,
                    project_root.as_deref(),
                    cli.all_projects,
                    provider.to_string(),
                    true,
                    view_mode.verbose,
                    cli.format,
                    &view_mode,
                ),
                IndexCommand::Vacuum { view_mode } => {
                    handlers::index::handle_vacuum(&workspace, cli.format, &view_mode)
                }
            }
        }

        Commands::Session { command } => {
            let workspace = AgTrace::open(data_dir.clone())?;

            match command {
                SessionCommand::List {
                    project_hash,
                    source,
                    limit,
                    since,
                    until,
                    no_auto_refresh,
                    format,
                    view_mode,
                } => {
                    let effective_hash = if project_hash.is_none() {
                        project_root.as_ref().map(|p| {
                            agtrace_types::project_hash_from_root(&p.display().to_string())
                        })
                    } else {
                        project_hash
                    };

                    handlers::session_list::handle(
                        &workspace,
                        project_root.as_deref(),
                        cli.all_projects,
                        effective_hash,
                        limit,
                        format,
                        source.map(|s| s.to_string()),
                        since.clone(),
                        until.clone(),
                        no_auto_refresh,
                        &view_mode,
                    )
                }
                SessionCommand::Show {
                    session_id,
                    format,
                    view_mode,
                } => handlers::session_show::handle(&workspace, session_id, format, &view_mode),
            }
        }

        Commands::Provider { command } => {
            let config_path = data_dir.join("config.toml");

            match command {
                ProviderCommand::List { view_mode } => {
                    handlers::provider::list(&config_path, cli.format, &view_mode)
                }
                ProviderCommand::Detect { view_mode } => {
                    handlers::provider::detect(&config_path, cli.format, &view_mode)
                }
                ProviderCommand::Set {
                    provider,
                    log_root,
                    enable,
                    disable,
                    view_mode,
                } => handlers::provider::set(
                    provider,
                    log_root,
                    enable,
                    disable,
                    &config_path,
                    cli.format,
                    &view_mode,
                ),
            }
        }

        Commands::Doctor { command } => match command {
            DoctorCommand::Run {
                provider,
                verbose,
                view_mode,
            } => {
                let workspace = AgTrace::open(data_dir)?;
                handlers::doctor_run::handle(
                    &workspace,
                    provider.to_string(),
                    verbose,
                    cli.format,
                    &view_mode,
                )
            }
            DoctorCommand::Inspect {
                file_path,
                lines,
                format,
                view_mode,
            } => handlers::doctor_inspect::handle(file_path, lines, format, cli.format, &view_mode),
            DoctorCommand::Check {
                file_path,
                provider,
                view_mode,
            } => handlers::doctor_check::handle(
                file_path,
                provider.map(|p| p.to_string()),
                cli.format,
                &view_mode,
            ),
        },

        Commands::Project { command } => {
            let workspace = AgTrace::open(data_dir)?;

            match command {
                ProjectCommand::List {
                    project_root: proj_root,
                    view_mode,
                } => handlers::project::handle(&workspace, proj_root, cli.format, &view_mode),
            }
        }

        Commands::Lab { command } => {
            let workspace = AgTrace::open(data_dir)?;

            match command {
                LabCommand::Export {
                    session_id,
                    output,
                    format,
                    strategy,
                } => {
                    let default_view_mode = crate::args::ViewModeArgs {
                        quiet: false,
                        compact: false,
                        verbose: false,
                    };
                    handlers::lab_export::handle(
                        &workspace,
                        session_id,
                        output,
                        format,
                        strategy,
                        cli.format,
                        &default_view_mode,
                    )
                }
                LabCommand::Stats { limit, source } => {
                    let default_view_mode = crate::args::ViewModeArgs {
                        quiet: false,
                        compact: false,
                        verbose: false,
                    };
                    handlers::lab_stats::handle(
                        &workspace,
                        limit,
                        source,
                        cli.format,
                        &default_view_mode,
                    )
                }
                LabCommand::Grep {
                    pattern,
                    limit,
                    source,
                    json,
                    raw,
                    regex,
                    r#type,
                    tool,
                    ignore_case,
                } => {
                    let options = handlers::lab_grep::GrepOptions {
                        pattern,
                        limit,
                        source,
                        json_output: json,
                        raw_output: raw,
                        use_regex: regex,
                        ignore_case,
                        event_type: r#type,
                        tool_name: tool,
                    };
                    let default_view_mode = crate::args::ViewModeArgs {
                        quiet: false,
                        compact: false,
                        verbose: false,
                    };
                    handlers::lab_grep::handle(&workspace, options, cli.format, &default_view_mode)
                }
            }
        }

        Commands::Sessions {
            project_hash,
            source,
            limit,
            since,
            until,
        } => {
            let workspace = AgTrace::open(data_dir)?;

            let effective_hash = if project_hash.is_none() {
                project_root
                    .as_ref()
                    .map(|p| agtrace_types::project_hash_from_root(&p.display().to_string()))
            } else {
                project_hash
            };

            // Create default ViewModeArgs for Sessions alias
            let default_view_mode = crate::args::ViewModeArgs {
                quiet: false,
                compact: false,
                verbose: false,
            };

            handlers::session_list::handle(
                &workspace,
                project_root.as_deref(),
                cli.all_projects,
                effective_hash,
                limit,
                cli.format,
                source.map(|s| s.to_string()),
                since,
                until,
                false, // no_auto_refresh - default to auto-refresh for Sessions command
                &default_view_mode,
            )
        }

        Commands::Pack { template, limit } => {
            let workspace = AgTrace::open(data_dir)?;
            let project_hash = project_root
                .as_ref()
                .map(|p| agtrace_types::project_hash_from_root(&p.display().to_string()));

            let default_view_mode = crate::args::ViewModeArgs {
                quiet: false,
                compact: false,
                verbose: false,
            };
            handlers::pack::handle(
                &workspace,
                &template.to_string(),
                limit,
                project_hash,
                cli.all_projects,
                cli.format,
                &default_view_mode,
            )
        }

        Commands::Watch { provider, id, mode } => {
            use crate::args::WatchFormat;

            // TUI mode requires TTY
            if mode == WatchFormat::Tui && !std::io::stdout().is_terminal() {
                anyhow::bail!("watch --mode tui requires a TTY (interactive terminal). Use --mode console for non-interactive streaming.");
            }

            let workspace = AgTrace::open(data_dir)?;

            let target = if let Some(session_id) = id {
                handlers::watch_tui::WatchTarget::Session { id: session_id }
            } else {
                let provider_name = if let Some(name) = provider {
                    name.to_string()
                } else {
                    // Get default provider from workspace config
                    workspace
                        .config()
                        .enabled_providers()
                        .into_iter()
                        .next()
                        .map(|(name, _)| name.clone())
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "No enabled provider found. Run 'agtrace init' to configure providers."
                            )
                        })?
                };
                handlers::watch_tui::WatchTarget::Provider {
                    name: provider_name,
                }
            };

            match mode {
                WatchFormat::Tui => {
                    handlers::watch_tui::handle(&workspace, project_root.as_deref(), target)
                }
                WatchFormat::Console => handlers::watch_console::handle_console(
                    &workspace,
                    project_root.as_deref(),
                    target,
                ),
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
