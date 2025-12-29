use super::args::{
    Cli, Commands, DoctorCommand, IndexCommand, LabCommand, ProjectCommand, ProviderCommand,
    SessionCommand, ViewModeArgs,
};
use super::handlers;
use agtrace_runtime::AgTrace;
use agtrace_types::project_hash_from_root;
use anyhow::Result;
use clap::CommandFactory;
use is_terminal::IsTerminal;
use std::path::PathBuf;

struct CommandContext {
    data_dir: PathBuf,
    project_root: Option<PathBuf>,
    all_projects: bool,
    format: crate::args::OutputFormat,
}

impl CommandContext {
    fn from_cli(cli: &Cli) -> Self {
        let data_dir = expand_tilde(&cli.data_dir);
        let project_root = cli
            .project_root
            .as_ref()
            .map(PathBuf::from)
            .or_else(|| std::env::current_dir().ok());

        Self {
            data_dir,
            project_root,
            all_projects: cli.all_projects,
            format: cli.format,
        }
    }

    fn open_workspace(&self) -> Result<AgTrace> {
        AgTrace::open(self.data_dir.clone())
    }

    fn config_path(&self) -> PathBuf {
        self.data_dir.join("config.toml")
    }

    fn project_hash(&self) -> Option<String> {
        self.project_root
            .as_ref()
            .map(|p| project_hash_from_root(&p.display().to_string()))
    }

    fn effective_project_hash(&self, explicit_hash: Option<String>) -> Option<String> {
        if self.all_projects {
            None
        } else {
            explicit_hash.or_else(|| self.project_hash())
        }
    }
}

fn default_view_mode() -> ViewModeArgs {
    ViewModeArgs::default()
}

pub fn run(cli: Cli) -> Result<()> {
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

        Commands::Index { command } => {
            let workspace = ctx.open_workspace()?;
            match command {
                IndexCommand::Update {
                    provider,
                    view_mode,
                } => handlers::index::handle(
                    &workspace,
                    ctx.project_root.as_deref(),
                    ctx.all_projects,
                    provider.to_string(),
                    false,
                    view_mode.verbose,
                    ctx.format,
                    &view_mode,
                ),
                IndexCommand::Rebuild {
                    provider,
                    view_mode,
                } => handlers::index::handle(
                    &workspace,
                    ctx.project_root.as_deref(),
                    ctx.all_projects,
                    provider.to_string(),
                    true,
                    view_mode.verbose,
                    ctx.format,
                    &view_mode,
                ),
                IndexCommand::Vacuum { view_mode } => {
                    handlers::index::handle_vacuum(&workspace, ctx.format, &view_mode)
                }
            }
        }

        Commands::Session { command } => {
            let workspace = ctx.open_workspace()?;
            match command {
                SessionCommand::List {
                    project_hash,
                    provider,
                    limit,
                    since,
                    until,
                    no_auto_refresh,
                    format,
                    view_mode,
                } => handlers::session_list::handle(
                    &workspace,
                    ctx.project_root.as_deref(),
                    ctx.all_projects,
                    ctx.effective_project_hash(project_hash),
                    limit,
                    format,
                    provider.map(|s| s.to_string()),
                    since,
                    until,
                    no_auto_refresh,
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
                verbose,
                view_mode,
            } => {
                let workspace = ctx.open_workspace()?;
                handlers::doctor_run::handle(
                    &workspace,
                    provider.to_string(),
                    verbose,
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
            } => handlers::doctor_check::handle(
                file_path,
                provider.map(|p| p.to_string()),
                ctx.format,
                &view_mode,
            ),
        },

        Commands::Project { command } => {
            let workspace = ctx.open_workspace()?;
            match command {
                ProjectCommand::List {
                    project_root: proj_root,
                    view_mode,
                } => handlers::project::handle(&workspace, proj_root, ctx.format, &view_mode),
            }
        }

        Commands::Lab { command } => {
            let workspace = ctx.open_workspace()?;
            match command {
                LabCommand::Export {
                    session_id,
                    output,
                    format,
                    strategy,
                } => handlers::lab_export::handle(
                    &workspace,
                    session_id,
                    output,
                    format,
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
        } => {
            let workspace = ctx.open_workspace()?;
            handlers::session_list::handle(
                &workspace,
                ctx.project_root.as_deref(),
                ctx.all_projects,
                ctx.effective_project_hash(project_hash),
                limit,
                ctx.format,
                provider.map(|s| s.to_string()),
                since,
                until,
                false, // no_auto_refresh - default to auto-refresh for Sessions command
                &default_view_mode(),
            )
        }

        Commands::Pack { template, limit } => {
            let workspace = ctx.open_workspace()?;
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

        Commands::Watch { provider, id, mode } => {
            use crate::args::WatchFormat;

            if mode == WatchFormat::Tui && !std::io::stdout().is_terminal() {
                anyhow::bail!(
                    "watch --mode tui requires a TTY (interactive terminal). Use --mode console for non-interactive streaming."
                );
            }

            let workspace = ctx.open_workspace()?;

            let target = if let Some(session_id) = id {
                handlers::watch_tui::WatchTarget::Session { id: session_id }
            } else {
                let provider_name = provider
                    .map(|p| p.to_string())
                    .or_else(|| {
                        find_provider_with_most_recent_session(&workspace, ctx.project_root.as_deref())
                    })
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "No sessions found in any enabled provider. Run 'agtrace init' to index your sessions."
                        )
                    })?;
                handlers::watch_tui::WatchTarget::Provider {
                    name: provider_name,
                }
            };

            match mode {
                WatchFormat::Tui => {
                    handlers::watch_tui::handle(&workspace, ctx.project_root.as_deref(), target)
                }
                WatchFormat::Console => handlers::watch_console::handle_console(
                    &workspace,
                    ctx.project_root.as_deref(),
                    target,
                ),
            }
        }
    }
}

// NOTE: Design rationale for finding most recent session
// - Watch mode needs real-time detection of "most recently updated" sessions
// - Cannot rely on DB indexing since watch bypasses DB for real-time monitoring
// - Directly scans filesystem using LogDiscovery::scan_sessions()
// - Uses SessionIndex.latest_mod_time (file modification time) instead of timestamp (creation time)
// - This enables switching to sessions that are actively being updated, even if created earlier
fn find_provider_with_most_recent_session(
    workspace: &agtrace_runtime::AgTrace,
    _project_root: Option<&std::path::Path>,
) -> Option<String> {
    let enabled_providers = workspace.config().enabled_providers();
    if enabled_providers.is_empty() {
        return None;
    }

    // Track the most recently updated session across all providers
    let mut most_recent: Option<(String, String)> = None; // (provider_name, latest_mod_time)

    for (provider_name, provider_config) in enabled_providers {
        // Create adapter for this provider
        let adapter = match agtrace_providers::create_adapter(provider_name) {
            Ok(a) => a,
            Err(_) => continue,
        };

        // Scan filesystem directly (bypassing DB for real-time detection)
        let sessions = match adapter.discovery.scan_sessions(&provider_config.log_root) {
            Ok(s) => s,
            Err(_) => continue,
        };

        // Find the session with the latest modification time in this provider
        for session in sessions {
            if let Some(ref mod_time) = session.latest_mod_time
                && (most_recent.is_none() || Some(mod_time) > most_recent.as_ref().map(|(_, t)| t))
            {
                most_recent = Some((provider_name.clone(), mod_time.clone()));
            }
        }
    }

    most_recent.map(|(provider, _)| provider)
}

fn expand_tilde(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/")
        && let Some(home) = std::env::var_os("HOME")
    {
        return PathBuf::from(home).join(stripped);
    }
    PathBuf::from(path)
}
