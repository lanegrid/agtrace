use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "agtrace")]
#[command(about = "Normalize and analyze agent behavior logs", long_about = None)]
#[command(version)]
pub struct Cli {
    #[arg(long, default_value = "~/.agtrace", global = true)]
    pub data_dir: String,

    #[arg(long, value_parser = ["plain", "json"], default_value = "plain", global = true)]
    pub format: String,

    #[arg(long, value_parser = ["error", "warn", "info", "debug", "trace"], default_value = "info", global = true)]
    pub log_level: String,

    #[arg(long, global = true)]
    pub project_root: Option<String>,

    #[arg(long, global = true)]
    pub all_projects: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Scan {
        #[arg(long, value_parser = ["claude", "codex", "gemini", "all"], default_value = "all")]
        provider: String,

        #[arg(long)]
        force: bool,

        #[arg(long)]
        verbose: bool,
    },

    List {
        #[arg(long)]
        project_hash: Option<String>,

        #[arg(long, value_parser = ["claude", "codex", "gemini"])]
        source: Option<String>,

        #[arg(long, default_value = "50")]
        limit: usize,

        #[arg(long)]
        since: Option<String>,

        #[arg(long)]
        until: Option<String>,
    },

    View {
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

        #[arg(long, help = "Display full event text without truncation (default behavior, kept for backwards compatibility)")]
        full: bool,

        #[arg(long, help = "Truncate long text to ~100 chars for compact display")]
        short: bool,
    },

    Show {
        session_id: String,

        #[arg(long)]
        event_type: Option<String>,

        #[arg(long)]
        no_reasoning: bool,

        #[arg(long)]
        no_tool: bool,

        #[arg(long)]
        limit: Option<usize>,
    },

    Find {
        #[arg(long)]
        session_id: Option<String>,

        #[arg(long)]
        project_hash: Option<String>,

        #[arg(long)]
        event_id: Option<String>,

        #[arg(long)]
        text: Option<String>,

        #[arg(long)]
        event_type: Option<String>,

        #[arg(long, default_value = "50")]
        limit: usize,
    },

    Stats {
        #[arg(long)]
        project_hash: Option<String>,

        #[arg(long, value_parser = ["claude", "codex", "gemini"])]
        source: Option<String>,

        #[arg(long, value_parser = ["project", "session", "source"])]
        group_by: Option<String>,

        #[arg(long)]
        since: Option<String>,

        #[arg(long)]
        until: Option<String>,
    },

    Export {
        #[arg(long)]
        project_hash: Option<String>,

        #[arg(long)]
        session_id: Option<String>,

        #[arg(long, value_parser = ["claude", "codex", "gemini"])]
        source: Option<String>,

        #[arg(long)]
        event_type: Option<String>,

        #[arg(long)]
        since: Option<String>,

        #[arg(long)]
        until: Option<String>,

        #[arg(long)]
        out: PathBuf,

        #[arg(long, value_parser = ["jsonl", "csv"], default_value = "jsonl")]
        format: String,
    },

    Providers {
        #[command(subcommand)]
        command: Option<ProvidersCommand>,
    },

    Project {
        #[arg(long)]
        project_root: Option<String>,
    },

    Diagnose {
        #[arg(long, value_parser = ["claude", "codex", "gemini", "all"], default_value = "all")]
        provider: String,

        #[arg(long)]
        verbose: bool,
    },

    Inspect {
        file_path: String,

        #[arg(long, default_value = "50")]
        lines: usize,

        #[arg(long, value_parser = ["raw", "json"], default_value = "raw")]
        format: String,
    },

    Validate {
        file_path: String,

        #[arg(long, value_parser = ["claude", "codex", "gemini"])]
        provider: Option<String>,
    },

    Schema {
        provider: String,

        #[arg(long, value_parser = ["text", "json", "rust"], default_value = "text")]
        format: String,
    },
}

#[derive(Subcommand)]
pub enum ProvidersCommand {
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
}
