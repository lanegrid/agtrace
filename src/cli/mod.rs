use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod commands;
mod import;
mod output;

pub use commands::run;

#[derive(Parser)]
#[command(name = "agtrace")]
#[command(about = "Normalize and analyze agent behavior logs", long_about = None)]
#[command(version)]
pub struct Cli {
    /// Data directory for storing normalized events
    #[arg(long, default_value = "~/.agtrace", global = true)]
    pub data_dir: String,

    /// Output format
    #[arg(long, value_parser = ["plain", "json"], default_value = "plain", global = true)]
    pub format: String,

    /// Log level
    #[arg(long, value_parser = ["error", "warn", "info", "debug", "trace"], default_value = "info", global = true)]
    pub log_level: String,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Import vendor logs and normalize them
    Import {
        /// Vendor source type (default: all enabled providers)
        #[arg(long, value_parser = ["claude", "codex", "gemini", "all"], default_value = "all")]
        source: String,

        /// Root directory of vendor logs (overrides config log_root)
        #[arg(long)]
        root: Option<PathBuf>,

        /// Project root path (optional override)
        #[arg(long)]
        project_root: Option<String>,

        /// Session ID prefix (codex only)
        #[arg(long)]
        session_id_prefix: Option<String>,

        /// Dry run (don't write to storage)
        #[arg(long)]
        dry_run: bool,

        /// Output JSONL file path
        #[arg(long)]
        out_jsonl: Option<PathBuf>,
    },

    /// List sessions
    List {
        /// Filter by project hash
        #[arg(long)]
        project_hash: Option<String>,

        /// Filter by source
        #[arg(long, value_parser = ["claude", "codex", "gemini"])]
        source: Option<String>,

        /// Maximum number of sessions to show
        #[arg(long, default_value = "50")]
        limit: usize,

        /// Filter by start time (RFC3339)
        #[arg(long)]
        since: Option<String>,

        /// Filter by end time (RFC3339)
        #[arg(long)]
        until: Option<String>,
    },

    /// Show session details
    Show {
        /// Session ID
        session_id: String,

        /// Filter by event types (comma-separated)
        #[arg(long)]
        event_type: Option<String>,

        /// Hide reasoning events
        #[arg(long)]
        no_reasoning: bool,

        /// Hide tool events
        #[arg(long)]
        no_tool: bool,

        /// Maximum number of events to show
        #[arg(long)]
        limit: Option<usize>,
    },

    /// Find events
    Find {
        /// Session ID
        #[arg(long)]
        session_id: Option<String>,

        /// Project hash
        #[arg(long)]
        project_hash: Option<String>,

        /// Event ID
        #[arg(long)]
        event_id: Option<String>,

        /// Text search query
        #[arg(long)]
        text: Option<String>,

        /// Event type filter
        #[arg(long)]
        event_type: Option<String>,

        /// Maximum results
        #[arg(long, default_value = "50")]
        limit: usize,
    },

    /// Show statistics
    Stats {
        /// Project hash
        #[arg(long)]
        project_hash: Option<String>,

        /// Source filter
        #[arg(long, value_parser = ["claude", "codex", "gemini"])]
        source: Option<String>,

        /// Group by field
        #[arg(long, value_parser = ["project", "session", "source"])]
        group_by: Option<String>,

        /// Since time (RFC3339)
        #[arg(long)]
        since: Option<String>,

        /// Until time (RFC3339)
        #[arg(long)]
        until: Option<String>,
    },

    /// Export events
    Export {
        /// Project hash
        #[arg(long)]
        project_hash: Option<String>,

        /// Session ID
        #[arg(long)]
        session_id: Option<String>,

        /// Source filter
        #[arg(long, value_parser = ["claude", "codex", "gemini"])]
        source: Option<String>,

        /// Event type filter
        #[arg(long)]
        event_type: Option<String>,

        /// Since time (RFC3339)
        #[arg(long)]
        since: Option<String>,

        /// Until time (RFC3339)
        #[arg(long)]
        until: Option<String>,

        /// Output file path
        #[arg(long)]
        out: PathBuf,

        /// Output format
        #[arg(long, value_parser = ["jsonl", "csv"], default_value = "jsonl")]
        format: String,
    },

    /// Manage provider configurations
    Providers {
        #[command(subcommand)]
        command: Option<ProvidersCommand>,
    },

    /// Show project information
    Project {
        /// Project root path (optional override)
        #[arg(long)]
        project_root: Option<String>,
    },

    /// Show project and session diagnostics
    Status {
        /// Project root path (optional override)
        #[arg(long)]
        project_root: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum ProvidersCommand {
    /// List all providers
    List,

    /// Detect providers automatically
    Detect,

    /// Set provider configuration
    Set {
        /// Provider name
        provider: String,

        /// Log root directory
        #[arg(long)]
        log_root: PathBuf,

        /// Enable the provider
        #[arg(long)]
        enable: bool,

        /// Disable the provider
        #[arg(long)]
        disable: bool,
    },
}
