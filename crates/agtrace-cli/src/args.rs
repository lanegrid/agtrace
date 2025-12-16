use crate::types::{
    ExportFormat, ExportStrategy, InspectFormat, LogLevel, OutputFormat, PackTemplate,
    ProviderFilter, ProviderName, SchemaFormat, ViewStyle,
};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

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
    Index {
        #[command(subcommand)]
        command: IndexCommand,
    },

    Session {
        #[command(subcommand)]
        command: SessionCommand,
    },

    Provider {
        #[command(subcommand)]
        command: ProviderCommand,
    },

    Doctor {
        #[command(subcommand)]
        command: DoctorCommand,
    },

    Project {
        #[command(subcommand)]
        command: ProjectCommand,
    },

    Lab {
        #[command(subcommand)]
        command: LabCommand,
    },

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

    Pack {
        #[arg(long, default_value = "compact")]
        template: PackTemplate,

        #[arg(long, default_value = "20")]
        limit: usize,
    },

    Watch {
        #[arg(long)]
        provider: Option<ProviderName>,

        #[arg(
            long,
            help = "Explicit session ID or file path to watch (bypasses liveness detection)"
        )]
        id: Option<String>,
    },

    Init {
        #[arg(long)]
        refresh: bool,
    },
}

#[derive(Subcommand)]
pub enum IndexCommand {
    Update {
        #[arg(long, default_value = "all")]
        provider: ProviderFilter,

        #[arg(long)]
        verbose: bool,
    },

    Rebuild {
        #[arg(long, default_value = "all")]
        provider: ProviderFilter,

        #[arg(long)]
        verbose: bool,
    },

    Vacuum,
}

#[derive(Subcommand)]
pub enum SessionCommand {
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
    },

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
    List,
    Detect,
    Set {
        provider: String,

        #[arg(long)]
        log_root: PathBuf,

        #[arg(long)]
        enable: bool,

        #[arg(long)]
        disable: bool,
    },
    Schema {
        provider: String,

        #[arg(long, default_value = "text")]
        format: SchemaFormat,
    },
}

#[derive(Subcommand)]
pub enum DoctorCommand {
    Run {
        #[arg(long, default_value = "all")]
        provider: ProviderFilter,

        #[arg(long)]
        verbose: bool,
    },

    Inspect {
        file_path: String,

        #[arg(long, default_value = "50")]
        lines: usize,

        #[arg(long, default_value = "raw")]
        format: InspectFormat,
    },

    Check {
        file_path: String,

        #[arg(long)]
        provider: Option<ProviderName>,
    },
}

#[derive(Subcommand)]
pub enum ProjectCommand {
    List {
        #[arg(long)]
        project_root: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum LabCommand {
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
