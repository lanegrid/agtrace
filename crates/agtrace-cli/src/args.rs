use crate::types::{
    ExportFormat, ExportStrategy, InspectFormat, LogLevel, OutputFormat, PackTemplate,
    ProviderFilter, ProviderName, SchemaFormat, ViewStyle,
};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

// NOTE: Command Organization Rationale
//
// Why namespaced subcommands (not flat)?
// - Flat command structures become unwieldy past ~10 commands
// - Namespaces (index, session, provider, doctor, lab, project) group related operations
// - Improves --help discoverability and conceptual clarity
// - Example: `session show` vs `session list` vs flat `show-session` and `list-sessions`

#[derive(Parser)]
#[command(name = "agtrace")]
#[command(about = "Normalize and analyze agent behavior logs", long_about = None)]
#[command(version)]
pub struct Cli {
    #[arg(long, default_value = "~/.agtrace", global = true)]
    pub data_dir: String,

    #[arg(long, default_value = "plain", global = true)]
    pub format: OutputFormat,

    #[arg(long, default_value = "info", global = true)]
    pub log_level: LogLevel,

    #[arg(long, global = true)]
    pub project_root: Option<String>,

    #[arg(long, global = true)]
    pub all_projects: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Manage index database operations")]
    Index {
        #[command(subcommand)]
        command: IndexCommand,
    },

    #[command(about = "Manage and view session data")]
    Session {
        #[command(subcommand)]
        command: SessionCommand,
    },

    #[command(about = "Manage provider configurations")]
    Provider {
        #[command(subcommand)]
        command: ProviderCommand,
    },

    #[command(about = "Diagnose provider configurations and log files")]
    Doctor {
        #[command(subcommand)]
        command: DoctorCommand,
    },

    #[command(about = "List indexed projects")]
    Project {
        #[command(subcommand)]
        command: ProjectCommand,
    },

    #[command(about = "Experimental features")]
    Lab {
        #[command(subcommand)]
        command: LabCommand,
    },

    #[command(about = "List recent sessions (alias for 'session list')")]
    Sessions {
        #[arg(long)]
        project_hash: Option<String>,

        #[arg(long)]
        source: Option<ProviderName>,

        #[arg(long, default_value = "50")]
        limit: usize,

        #[arg(long)]
        since: Option<String>,

        #[arg(long)]
        until: Option<String>,
    },

    #[command(about = "Analyze and pack sessions for sharing")]
    Pack {
        #[arg(long, default_value = "compact")]
        template: PackTemplate,

        #[arg(long, default_value = "20")]
        limit: usize,
    },

    #[command(about = "Watch for live session updates")]
    Watch {
        #[arg(long)]
        provider: Option<ProviderName>,

        #[arg(
            long,
            help = "Explicit session ID or file path to watch (bypasses liveness detection)"
        )]
        id: Option<String>,

        #[arg(long, help = "Use refreshing UI with context window at bottom")]
        refresh: bool,
    },

    #[command(about = "Initialize agtrace configuration")]
    Init {
        #[arg(long)]
        refresh: bool,
    },
}

#[derive(Subcommand)]
pub enum IndexCommand {
    #[command(about = "Incrementally update the index with new sessions")]
    Update {
        #[arg(long, default_value = "all")]
        provider: ProviderFilter,

        #[arg(long)]
        verbose: bool,
    },

    #[command(about = "Rebuild the entire index from scratch")]
    Rebuild {
        #[arg(long, default_value = "all")]
        provider: ProviderFilter,

        #[arg(long)]
        verbose: bool,
    },

    #[command(about = "Optimize database by reclaiming unused space")]
    Vacuum,
}

#[derive(Subcommand)]
pub enum SessionCommand {
    #[command(about = "List recent sessions with filtering options")]
    List {
        #[arg(long)]
        project_hash: Option<String>,

        #[arg(long)]
        source: Option<ProviderName>,

        #[arg(long, default_value = "50")]
        limit: usize,

        #[arg(long)]
        since: Option<String>,

        #[arg(long)]
        until: Option<String>,

        #[arg(long)]
        no_auto_refresh: bool,
    },

    #[command(about = "Display detailed session timeline and events")]
    Show {
        session_id: String,

        #[arg(long)]
        raw: bool,

        #[arg(long)]
        json: bool,

        #[arg(long)]
        timeline: bool,

        #[arg(long, value_delimiter = ',')]
        hide: Option<Vec<String>>,

        #[arg(long, value_delimiter = ',')]
        only: Option<Vec<String>>,

        #[arg(
            long,
            help = "Display full event text without truncation (default behavior, kept for backwards compatibility)"
        )]
        full: bool,

        #[arg(long, help = "Truncate long text to ~100 chars for compact display")]
        short: bool,

        #[arg(long, default_value = "timeline")]
        style: ViewStyle,
    },
}

#[derive(Subcommand)]
pub enum ProviderCommand {
    #[command(about = "Show configured providers")]
    List,
    #[command(about = "Auto-detect and configure providers")]
    Detect,
    #[command(about = "Configure a provider")]
    Set {
        provider: String,

        #[arg(long)]
        log_root: PathBuf,

        #[arg(long)]
        enable: bool,

        #[arg(long)]
        disable: bool,
    },
    #[command(about = "Show provider event schema")]
    Schema {
        provider: String,

        #[arg(long, default_value = "text")]
        format: SchemaFormat,
    },
}

#[derive(Subcommand)]
pub enum DoctorCommand {
    #[command(about = "Diagnose provider log files for parse errors")]
    Run {
        #[arg(long, default_value = "all")]
        provider: ProviderFilter,

        #[arg(long)]
        verbose: bool,
    },

    #[command(about = "Inspect raw log file contents")]
    Inspect {
        file_path: String,

        #[arg(long, default_value = "50")]
        lines: usize,

        #[arg(long, default_value = "raw")]
        format: InspectFormat,
    },

    #[command(about = "Check if a log file can be parsed")]
    Check {
        file_path: String,

        #[arg(long)]
        provider: Option<ProviderName>,
    },
}

#[derive(Subcommand)]
pub enum ProjectCommand {
    #[command(about = "List all indexed projects")]
    List {
        #[arg(long)]
        project_root: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum LabCommand {
    #[command(about = "Export session data to various formats")]
    Export {
        session_id: String,

        #[arg(long)]
        output: Option<PathBuf>,

        #[arg(long, default_value = "jsonl")]
        format: ExportFormat,

        #[arg(long, default_value = "raw")]
        strategy: ExportStrategy,
    },
}
