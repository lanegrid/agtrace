use super::args::{
    Cli, Commands, DoctorCommand, IndexCommand, LabCommand, ProjectCommand, ProviderCommand,
    SessionCommand, ViewModeArgs, WorktreeCommand,
};
use super::handlers;
use agtrace_sdk::Client;
use anyhow::Result;
use clap::CommandFactory;
use is_terminal::IsTerminal;
use std::path::PathBuf;

struct CommandContext {
    data_dir: PathBuf,
    project_root: Option<PathBuf>,
    all_projects: bool,
    all_worktrees: bool,
    format: crate::args::OutputFormat,
}

impl CommandContext {
    fn from_cli(cli: &Cli) -> Self {
        let data_dir = agtrace_sdk::utils::resolve_workspace_path(cli.data_dir.as_deref())
            .unwrap_or_else(|_| {
                // Fallback to working directory if resolution fails (should never happen)
                PathBuf::from(".agtrace")
            });
        let project_root = cli
            .project_root
            .as_ref()
            .map(PathBuf::from)
            .or_else(|| std::env::current_dir().ok());

        Self {
            data_dir,
            project_root,
            all_projects: cli.all_projects,
            all_worktrees: cli.all_worktrees,
            format: cli.format,
        }
    }

    async fn open_workspace(&self) -> Result<Client> {
        Ok(Client::connect(self.data_dir.clone()).await?)
    }

    fn config_path(&self) -> PathBuf {
        self.data_dir.join("config.toml")
    }

    fn project_hash(&self) -> Option<agtrace_sdk::types::ProjectHash> {
        self.project_root
            .as_ref()
            .map(|p| agtrace_sdk::utils::project_hash_from_root(&p.display().to_string()))
    }

    fn repository_hash(&self) -> Option<agtrace_sdk::types::RepositoryHash> {
        self.project_root
            .as_ref()
            .and_then(|p| agtrace_sdk::utils::repository_hash_from_path(p))
    }

    fn effective_project_hash(
        &self,
        explicit_hash: Option<String>,
    ) -> Option<agtrace_sdk::types::ProjectHash> {
        if self.all_projects || self.all_worktrees {
            // When --all-worktrees is set, we fetch all sessions and filter by repository_hash later
            None
        } else {
            explicit_hash
                .map(agtrace_sdk::types::ProjectHash::from)
                .or_else(|| self.project_hash())
        }
    }
}

fn default_view_mode() -> ViewModeArgs {
    ViewModeArgs::default()
}

pub async fn run(cli: Cli) -> Result<()> {
    let ctx = CommandContext::from_cli(&cli);

    let Some(command) = cli.command else {
        Cli::command().print_help()?;
        return Ok(());
    };

    match command {
        Commands::Init { refresh } => handlers::init::handle(
            &ctx.data_dir,
            ctx.project_root.clone(),
            ctx.all_projects,
            refresh,
            ctx.format,
            &default_view_mode(),
        ),

        Commands::Demo { speed } => handlers::demo::handle(speed),

        Commands::Index { command } => match command {
            IndexCommand::Info => {
                handlers::index::handle_info(&ctx.data_dir, ctx.format, &default_view_mode())
            }
            IndexCommand::Update {
                provider,
                view_mode,
            } => {
                let workspace = ctx.open_workspace().await?;
                handlers::index::handle(
                    &workspace,
                    ctx.project_root.as_deref(),
                    ctx.all_projects,
                    provider.to_string(),
                    false,
                    view_mode.verbose,
                    ctx.format,
                    &view_mode,
                )
            }
            IndexCommand::Rebuild {
                provider,
                view_mode,
            } => {
                let workspace = ctx.open_workspace().await?;
                handlers::index::handle(
                    &workspace,
                    ctx.project_root.as_deref(),
                    ctx.all_projects,
                    provider.to_string(),
                    true,
                    view_mode.verbose,
                    ctx.format,
                    &view_mode,
                )
            }
            IndexCommand::Vacuum { view_mode } => {
                let workspace = ctx.open_workspace().await?;
                handlers::index::handle_vacuum(&workspace, ctx.format, &view_mode)
            }
        },

        Commands::Session { command } => {
            let workspace = ctx.open_workspace().await?;
            match command {
                SessionCommand::List {
                    project_hash,
                    provider,
                    limit,
                    since,
                    until,
                    no_auto_refresh,
                    all,
                    format,
                    view_mode,
                } => handlers::session_list::handle(
                    &workspace,
                    ctx.project_root.as_deref(),
                    ctx.all_projects,
                    ctx.all_worktrees,
                    ctx.repository_hash(),
                    ctx.effective_project_hash(project_hash),
                    limit,
                    format,
                    provider.map(|s| s.to_string()),
                    since,
                    until,
                    no_auto_refresh,
                    all,
                    &view_mode,
                ),
                SessionCommand::Show {
                    session_id,
                    format,
                    view_mode,
                } => handlers::session_show::handle(&workspace, session_id, format, &view_mode),
            }
        }

        Commands::Provider { command } => {
            let config_path = ctx.config_path();
            match command {
                ProviderCommand::List { view_mode } => {
                    handlers::provider::list(&config_path, ctx.format, &view_mode)
                }
                ProviderCommand::Detect { view_mode } => {
                    handlers::provider::detect(&config_path, ctx.format, &view_mode)
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
                    ctx.format,
                    &view_mode,
                ),
            }
        }

        Commands::Doctor { command } => match command {
            DoctorCommand::Run {
                provider,
                view_mode,
            } => {
                let workspace = ctx.open_workspace().await?;
                handlers::doctor_run::handle(
                    &workspace,
                    provider.to_string(),
                    view_mode.verbose,
                    ctx.format,
                    &view_mode,
                )
            }
            DoctorCommand::Inspect {
                file_path,
                lines,
                format,
                view_mode,
            } => handlers::doctor_inspect::handle(file_path, lines, format, ctx.format, &view_mode),
            DoctorCommand::Check {
                file_path,
                provider,
                view_mode,
            } => {
                let workspace = ctx.open_workspace().await?;
                handlers::doctor_check::handle(
                    &workspace,
                    file_path,
                    provider.map(|p| p.to_string()),
                    ctx.format,
                    &view_mode,
                )
            }
        },

        Commands::Project { command } => {
            let workspace = ctx.open_workspace().await?;
            match command {
                ProjectCommand::List {
                    project_root: proj_root,
                    view_mode,
                } => handlers::project::handle(&workspace, proj_root, ctx.format, &view_mode),
            }
        }

        Commands::Worktree { command } => {
            let workspace = ctx.open_workspace().await?;
            match command {
                WorktreeCommand::List { format, view_mode } => handlers::worktree::handle_list(
                    &workspace,
                    ctx.repository_hash(),
                    format,
                    &view_mode,
                ),
                WorktreeCommand::Sessions {
                    limit,
                    provider,
                    format,
                    view_mode,
                } => handlers::worktree::handle_sessions(
                    &workspace,
                    ctx.repository_hash(),
                    limit,
                    provider.map(|p| p.to_string()),
                    format,
                    &view_mode,
                ),
            }
        }

        Commands::Lab { command } => {
            let workspace = ctx.open_workspace().await?;
            match command {
                LabCommand::Export {
                    session_id,
                    output,
                    export_format,
                    strategy,
                } => handlers::lab_export::handle(
                    &workspace,
                    session_id,
                    output,
                    export_format,
                    strategy,
                    ctx.format,
                    &default_view_mode(),
                ),
                LabCommand::Stats { limit, provider } => handlers::lab_stats::handle(
                    &workspace,
                    limit,
                    provider,
                    ctx.format,
                    &default_view_mode(),
                ),
                LabCommand::Grep {
                    pattern,
                    limit,
                    provider,
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
                        provider,
                        json_output: json,
                        raw_output: raw,
                        use_regex: regex,
                        ignore_case,
                        event_type: r#type,
                        tool_name: tool,
                    };
                    handlers::lab_grep::handle(
                        &workspace,
                        options,
                        ctx.format,
                        &default_view_mode(),
                    )
                }
            }
        }

        Commands::Sessions {
            project_hash,
            provider,
            limit,
            since,
            until,
            all,
        } => {
            let workspace = ctx.open_workspace().await?;
            handlers::session_list::handle(
                &workspace,
                ctx.project_root.as_deref(),
                ctx.all_projects,
                ctx.all_worktrees,
                ctx.repository_hash(),
                ctx.effective_project_hash(project_hash),
                limit,
                ctx.format,
                provider.map(|s| s.to_string()),
                since,
                until,
                false, // no_auto_refresh - default to auto-refresh for Sessions command
                all,
                &default_view_mode(),
            )
        }

        Commands::Pack { template, limit } => {
            let workspace = ctx.open_workspace().await?;
            handlers::pack::handle(
                &workspace,
                &template.to_string(),
                limit,
                ctx.project_hash(),
                ctx.all_projects,
                ctx.format,
                &default_view_mode(),
            )
        }

        Commands::Watch {
            provider,
            id,
            mode,
            debug,
        } => {
            use crate::args::WatchFormat;

            if mode == WatchFormat::Tui && !std::io::stdout().is_terminal() {
                anyhow::bail!(
                    "watch --mode tui requires a TTY (interactive terminal). Use --mode console for non-interactive streaming."
                );
            }

            let workspace = ctx.open_workspace().await?;

            let target = if let Some(session_id) = id {
                handlers::watch_tui::WatchTarget::Session { id: session_id }
            } else {
                let provider_name = provider
                    .map(|p| p.to_string())
                    .or_else(|| {
                        workspace
                            .watch_service()
                            .find_most_recent_provider(ctx.project_root.as_deref())
                    })
                    .or_else(|| {
                        // Fallback: Select first enabled provider from config
                        // This allows watch to start in waiting mode even when no sessions exist yet
                        workspace
                            .watch_service()
                            .config()
                            .providers
                            .iter()
                            .find(|(_, cfg)| cfg.enabled)
                            .map(|(name, _)| name.clone())
                    })
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "No enabled providers found. Run 'agtrace init' to setup providers."
                        )
                    })?;
                handlers::watch_tui::WatchTarget::Provider {
                    name: provider_name,
                }
            };

            match mode {
                WatchFormat::Tui => handlers::watch_tui::handle(
                    &workspace,
                    ctx.project_root.as_deref(),
                    target,
                    debug,
                ),
                WatchFormat::Console => handlers::watch_console::handle_console(
                    &workspace,
                    ctx.project_root.as_deref(),
                    target,
                ),
            }
        }

        Commands::Mcp { command } => {
            let workspace = ctx.open_workspace().await?;
            handlers::mcp::handle(&workspace, &command).await
        }
    }
}
