mod export;
mod find;
mod formatters;
mod list;
mod show;
mod stats;

use crate::error::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use export::cmd_export;
use find::cmd_find;
use list::cmd_list;
use show::cmd_show;
use stats::cmd_stats;

#[derive(Parser)]
#[command(name = "agtrace")]
#[command(about = "Unify session histories from AI coding agents", long_about = None)]
pub struct Cli {
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

        /// Filter by date (YYYY-MM-DD)
        #[arg(long)]
        since: Option<String>,

        /// Show all executions (default: 10)
        #[arg(long)]
        all: bool,

        /// Number of executions to show (default: 10)
        #[arg(long)]
        limit: Option<usize>,
    },

    /// Find and show details of an execution by ID (searches all agents)
    Find {
        /// Execution ID
        id: String,

        /// Include event timeline
        #[arg(long)]
        events: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
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

        /// Output as JSON
        #[arg(long)]
        json: bool,
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

        /// Output format
        #[arg(long, default_value = "json")]
        format: String,
    },
}

pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::List {
            agent,
            path,
            project,
            since,
            all,
            limit,
        } => cmd_list(agent, path, project, since, all, limit),
        Commands::Find { id, events, json } => cmd_find(&id, events, json),
        Commands::Show {
            agent,
            id,
            path,
            events,
            json,
        } => cmd_show(&agent, &id, path, events, json),
        Commands::Stats {
            agent,
            path,
            by_agent,
            by_project,
            by_day,
        } => cmd_stats(agent, path, by_agent, by_project, by_day),
        Commands::Export {
            agent,
            id,
            all,
            path,
            format,
        } => cmd_export(agent, id, all, path, &format),
    }
}
