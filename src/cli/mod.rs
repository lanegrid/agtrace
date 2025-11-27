mod export;
mod find;
mod formatters;
mod list;
mod show;
mod stats;

use crate::error::Result;
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

use export::cmd_export;
use find::cmd_find;
use list::cmd_list;
use show::cmd_show;
use stats::cmd_stats;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    Table,
    Json,
    Jsonl,
}

impl OutputFormat {
    pub fn is_json(&self) -> bool {
        matches!(self, OutputFormat::Json | OutputFormat::Jsonl)
    }
}

#[derive(Parser)]
#[command(name = "agtrace")]
#[command(about = "Unify session histories from AI coding agents", long_about = None)]
#[command(version)]
pub struct Cli {
    /// Increase verbosity (-v, -vv, -vvv)
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Disable all colored output
    #[arg(long, global = true)]
    pub no_color: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List all executions (reads directly from agent directories)
    List {
        /// Filter by agent type
        #[arg(long)]
        agent: Option<String>,

        /// Custom path to read from
        #[arg(long)]
        path: Option<PathBuf>,

        /// Filter by project path
        #[arg(long)]
        project: Option<PathBuf>,

        /// Filter by start date (YYYY-MM-DD)
        #[arg(long)]
        since: Option<String>,

        /// Filter by end date (YYYY-MM-DD)
        #[arg(long)]
        until: Option<String>,

        /// Filter by minimum duration in seconds
        #[arg(long)]
        min_duration: Option<u64>,

        /// Filter by maximum duration in seconds
        #[arg(long)]
        max_duration: Option<u64>,

        /// Show all executions (default: 10)
        #[arg(long)]
        all: bool,

        /// Number of executions to show (default: 10)
        #[arg(long)]
        limit: Option<usize>,

        /// Sort by field (started_at, duration, tokens)
        #[arg(long, default_value = "started_at")]
        sort: String,

        /// Reverse sort order
        #[arg(long)]
        reverse: bool,

        /// Output format
        #[arg(long, value_enum, default_value = "table")]
        format: OutputFormat,

        /// Suppress header and hints (useful for piping)
        #[arg(long)]
        quiet: bool,

        /// Suppress table header
        #[arg(long)]
        no_header: bool,

        /// Columns to display (comma-separated: id,agent,path,turns,duration,date,task)
        #[arg(long)]
        columns: Option<String>,
    },

    /// Find and show details of an execution by ID (searches all agents)
    Find {
        /// Execution ID
        id: String,

        /// Include event timeline
        #[arg(long)]
        events: bool,

        /// Limit number of events to show
        #[arg(long)]
        events_limit: Option<usize>,

        /// Output format
        #[arg(long, value_enum, default_value = "table")]
        format: OutputFormat,
    },

    /// Show details of a specific execution
    Show {
        /// Agent type (claude-code or codex)
        agent: String,

        /// Execution ID
        id: String,

        /// Custom path to read from
        #[arg(long)]
        path: Option<PathBuf>,

        /// Include event timeline
        #[arg(long)]
        events: bool,

        /// Limit number of events to show
        #[arg(long)]
        events_limit: Option<usize>,

        /// Output format
        #[arg(long, value_enum, default_value = "table")]
        format: OutputFormat,
    },

    /// Show statistics (computed on-the-fly from agent directories)
    Stats {
        /// Filter by agent type
        #[arg(long)]
        agent: Option<String>,

        /// Custom path to read from
        #[arg(long)]
        path: Option<PathBuf>,

        /// Group by agent
        #[arg(long)]
        by_agent: bool,

        /// Group by project
        #[arg(long)]
        by_project: bool,

        /// Group by day
        #[arg(long)]
        by_day: bool,

        /// Output format
        #[arg(long, value_enum, default_value = "table")]
        format: OutputFormat,
    },

    /// Export executions (reads directly and exports)
    Export {
        /// Agent type (claude-code or codex) - required if id is specified
        agent: Option<String>,

        /// Execution ID (optional, use --all to export all)
        id: Option<String>,

        /// Export all executions
        #[arg(long)]
        all: bool,

        /// Custom path to read from
        #[arg(long)]
        path: Option<PathBuf>,

        /// Filter by start date (YYYY-MM-DD)
        #[arg(long)]
        since: Option<String>,

        /// Filter by project path
        #[arg(long)]
        project: Option<PathBuf>,

        /// Number of executions to export (for --all)
        #[arg(long)]
        limit: Option<usize>,

        /// Output format
        #[arg(long, value_enum, default_value = "json")]
        format: OutputFormat,
    },

    /// Generate shell completion scripts
    Completions {
        /// Shell type
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

pub fn run(cli: Cli) -> Result<()> {
    // Initialize color support based on --no-color flag
    let use_color = !cli.no_color && std::env::var("NO_COLOR").is_err();

    match cli.command {
        Commands::List {
            agent,
            path,
            project,
            since,
            until,
            min_duration,
            max_duration,
            all,
            limit,
            sort,
            reverse,
            format,
            quiet,
            no_header,
            columns,
        } => cmd_list(
            agent,
            path,
            project,
            since,
            until,
            min_duration,
            max_duration,
            all,
            limit,
            sort,
            reverse,
            format,
            quiet,
            no_header,
            columns,
            use_color,
        ),
        Commands::Find {
            id,
            events,
            events_limit,
            format,
        } => cmd_find(&id, events, events_limit, format, use_color),
        Commands::Show {
            agent,
            id,
            path,
            events,
            events_limit,
            format,
        } => cmd_show(&agent, &id, path, events, events_limit, format, use_color),
        Commands::Stats {
            agent,
            path,
            by_agent,
            by_project,
            by_day,
            format,
        } => cmd_stats(agent, path, by_agent, by_project, by_day, format, use_color),
        Commands::Export {
            agent,
            id,
            all,
            path,
            since,
            project,
            limit,
            format,
        } => cmd_export(agent, id, all, path, since, project, limit, format),
        Commands::Completions { shell } => {
            use clap::CommandFactory;
            let mut cmd = Cli::command();
            clap_complete::generate(shell, &mut cmd, "agtrace", &mut std::io::stdout());
            Ok(())
        }
    }
}
