use super::args::{
    Cli, Commands, DoctorCommand, IndexCommand, LabCommand, ProjectCommand, ProviderCommand,
    SessionCommand,
};
use super::handlers;
use crate::context::ExecutionContext;
use crate::ui::models::GuidanceContext;
use crate::ui::{ConsoleTraceView, TraceView};
use agtrace_index::Database;
use anyhow::Result;
use std::path::{Path, PathBuf};

pub fn run(cli: Cli) -> Result<()> {
    let data_dir = expand_tilde(&cli.data_dir);
    let view = ConsoleTraceView::new();

    let Some(command) = cli.command else {
        // Check if we should show corpus overview or guidance
        let db_path = data_dir.join("agtrace.db");
        if db_path.exists() {
            if let Ok(db) = Database::open(&db_path) {
                if let Ok(sessions) = db.list_sessions(None, 1) {
                    if !sessions.is_empty() {
                        // Show corpus overview instead of guidance
                        let ctx = ExecutionContext::new(
                            data_dir.clone(),
                            cli.project_root.clone(),
                            cli.all_projects,
                        )?;
                        return handlers::corpus_overview::handle(&ctx, cli.project_root, &view);
                    }
                }
            }
        }
        show_guidance(&data_dir, &view)?;
        return Ok(());
    };

    match command {
        Commands::Init { refresh } => {
            let ctx = ExecutionContext::new(
                data_dir.clone(),
                cli.project_root.clone(),
                cli.all_projects,
            )?;
            handlers::init::handle(&ctx, refresh, &view)
        }

        Commands::Index { command } => {
            let ctx = ExecutionContext::new(
                data_dir.clone(),
                cli.project_root.clone(),
                cli.all_projects,
            )?;

            match command {
                IndexCommand::Update { provider, verbose } => {
                    handlers::index::handle(&ctx, provider.to_string(), false, verbose, &view)
                }
                IndexCommand::Rebuild { provider, verbose } => {
                    handlers::index::handle(&ctx, provider.to_string(), true, verbose, &view)
                }
                IndexCommand::Vacuum => {
                    let db = ctx.db()?;
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
                    no_auto_refresh,
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
                        cli.format,
                        source.map(|s| s.to_string()),
                        since.clone(),
                        until.clone(),
                        no_auto_refresh,
                        &data_dir,
                        cli.project_root.clone(),
                        &view,
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
                    &db, session_id, raw, json, timeline, hide, only, full, short, style, &view,
                ),
            }
        }

        Commands::Provider { command } => {
            let config_path = data_dir.join("config.toml");

            match command {
                ProviderCommand::List => handlers::provider::list(&config_path, &view),
                ProviderCommand::Detect => handlers::provider::detect(&config_path, &view),
                ProviderCommand::Set {
                    provider,
                    log_root,
                    enable,
                    disable,
                } => handlers::provider::set(
                    provider,
                    log_root,
                    enable,
                    disable,
                    &config_path,
                    &view,
                ),
            }
        }

        Commands::Doctor { command } => match command {
            DoctorCommand::Run { provider, verbose } => {
                let ctx = ExecutionContext::new(
                    data_dir.clone(),
                    cli.project_root.clone(),
                    cli.all_projects,
                )?;
                handlers::doctor_run::handle(&ctx, provider.to_string(), verbose, &view)
            }
            DoctorCommand::Inspect {
                file_path,
                lines,
                format,
            } => handlers::doctor_inspect::handle(file_path, lines, format, &view),
            DoctorCommand::Check {
                file_path,
                provider,
            } => handlers::doctor_check::handle(file_path, provider.map(|p| p.to_string()), &view),
        },

        Commands::Project { command } => {
            let ctx = ExecutionContext::new(
                data_dir.clone(),
                cli.project_root.clone(),
                cli.all_projects,
            )?;

            match command {
                ProjectCommand::List { project_root } => {
                    handlers::project::handle(&ctx, project_root, &view)
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
                } => handlers::lab_export::handle(&db, session_id, output, format, strategy, &view),
                LabCommand::Stats { limit, source } => {
                    handlers::lab_stats::handle(&db, limit, source, &view)
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
            let db_path = data_dir.join("agtrace.db");
            let db = Database::open(&db_path)?;

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
                cli.format,
                source.map(|s| s.to_string()),
                since,
                until,
                false, // no_auto_refresh - default to auto-refresh for Sessions command
                &data_dir,
                cli.project_root.clone(),
                &view,
            )
        }

        Commands::Pack { template, limit } => {
            let ctx = ExecutionContext::new(
                data_dir.clone(),
                cli.project_root.clone(),
                cli.all_projects,
            )?;

            handlers::pack::handle(&ctx, &template.to_string(), limit, cli.project_root, &view)
        }

        Commands::Watch {
            provider,
            id,
            refresh,
        } => {
            let ctx = ExecutionContext::new(data_dir, cli.project_root, cli.all_projects)?;

            let target = if let Some(session_id) = id {
                handlers::watch::WatchTarget::Session { id: session_id }
            } else {
                let provider_name = if let Some(name) = provider {
                    name.to_string()
                } else {
                    ctx.default_provider()?
                };
                handlers::watch::WatchTarget::Provider {
                    name: provider_name,
                }
            };

            if refresh {
                use crate::ui::{AnsiTerminal, RefreshingWatchView};
                let terminal = Box::new(AnsiTerminal::new());
                let refresh_view = RefreshingWatchView::new(terminal);
                handlers::watch::handle(&ctx, target, &refresh_view)
            } else {
                handlers::watch::handle(&ctx, target, &view)
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

fn show_guidance(data_dir: &Path, view: &dyn TraceView) -> Result<()> {
    let config_path = data_dir.join("config.toml");
    let db_path = data_dir.join("agtrace.db");

    let config_exists = config_path.exists();
    let db_exists = db_path.exists();

    let session_count = if config_exists && db_exists {
        let db = Database::open(&db_path)?;
        db.list_sessions(None, 1)?.len()
    } else {
        0
    };

    let context = GuidanceContext {
        config_exists,
        db_exists,
        session_count,
    };

    view.render_guidance(&context)
}
