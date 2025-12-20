use super::args::{
    Cli, Commands, DoctorCommand, IndexCommand, LabCommand, ProjectCommand, ProviderCommand,
    SessionCommand,
};
use super::handlers;
use crate::presentation::renderers::{ConsoleTraceView, TraceView};
use crate::presentation::view_models::GuidanceContext;
use agtrace_runtime::AgTrace;
use anyhow::Result;
use std::path::{Path, PathBuf};

pub fn run(cli: Cli) -> Result<()> {
    let data_dir = expand_tilde(&cli.data_dir);
    let view = ConsoleTraceView::new();

    let Some(command) = cli.command else {
        // Check if we should show corpus overview or guidance
        let db_path = data_dir.join("agtrace.db");
        if db_path.exists() {
            // Try to open workspace to check if we have sessions
            if let Ok(workspace) = AgTrace::open(data_dir.clone()) {
                if let Ok(sessions) = workspace
                    .sessions()
                    .list(agtrace_runtime::SessionFilter::new().limit(1))
                {
                    if !sessions.is_empty() {
                        // Show corpus overview instead of guidance
                        return handlers::corpus_overview::handle(
                            &workspace,
                            cli.project_root,
                            cli.all_projects,
                            &view,
                        );
                    }
                }
            }
        }
        show_guidance(&data_dir, &view)?;
        return Ok(());
    };

    let project_root = cli
        .project_root
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok());

    match command {
        Commands::Init { refresh } => handlers::init::handle(
            &data_dir,
            project_root.clone(),
            cli.all_projects,
            refresh,
            &view,
        ),

        Commands::Index { command } => {
            let workspace = AgTrace::open(data_dir.clone())?;

            match command {
                IndexCommand::Update { provider, verbose } => handlers::index::handle(
                    &workspace,
                    project_root.as_deref(),
                    cli.all_projects,
                    provider.to_string(),
                    false,
                    verbose,
                    &view,
                ),
                IndexCommand::Rebuild { provider, verbose } => handlers::index::handle(
                    &workspace,
                    project_root.as_deref(),
                    cli.all_projects,
                    provider.to_string(),
                    true,
                    verbose,
                    &view,
                ),
                IndexCommand::Vacuum => workspace.database().vacuum(),
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
                        cli.format,
                        source.map(|s| s.to_string()),
                        since.clone(),
                        until.clone(),
                        no_auto_refresh,
                        &view,
                    )
                }
                SessionCommand::Show {
                    session_id,
                    raw,
                    json,
                    hide,
                    only,
                    short,
                    verbose,
                } => handlers::session_show::handle(
                    &workspace, session_id, raw, json, hide, only, short, verbose, &view,
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
                let workspace = AgTrace::open(data_dir)?;
                handlers::doctor_run::handle(&workspace, provider.to_string(), verbose, &view)
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
            let workspace = AgTrace::open(data_dir)?;

            match command {
                ProjectCommand::List {
                    project_root: proj_root,
                } => handlers::project::handle(&workspace, proj_root, &view),
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
                } => handlers::lab_export::handle(
                    &workspace, session_id, output, format, strategy, &view,
                ),
                LabCommand::Stats { limit, source } => {
                    handlers::lab_stats::handle(&workspace, limit, source, &view)
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
                &view,
            )
        }

        Commands::Pack { template, limit } => {
            let workspace = AgTrace::open(data_dir)?;
            let project_hash = project_root
                .as_ref()
                .map(|p| agtrace_types::project_hash_from_root(&p.display().to_string()));

            handlers::pack::handle(
                &workspace,
                &template.to_string(),
                limit,
                project_hash,
                cli.all_projects,
                &view,
            )
        }

        Commands::Watch { provider, id } => {
            let workspace = AgTrace::open(data_dir)?;

            let target = if let Some(session_id) = id {
                handlers::watch::WatchTarget::Session { id: session_id }
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
                handlers::watch::WatchTarget::Provider {
                    name: provider_name,
                }
            };

            handlers::watch::handle(&workspace, project_root.as_deref(), target)
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
        // Try to open workspace to count sessions
        match agtrace_runtime::AgTrace::open(data_dir.to_path_buf()) {
            Ok(workspace) => {
                match workspace
                    .sessions()
                    .list(agtrace_runtime::SessionFilter::new().limit(1))
                {
                    Ok(sessions) => sessions.len(),
                    Err(_) => 0,
                }
            }
            Err(_) => 0,
        }
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
