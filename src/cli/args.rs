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

    Import {
        #[arg(long, value_parser = ["claude", "codex", "gemini", "all"], default_value = "all")]
        source: String,

        #[arg(long)]
        root: Option<PathBuf>,

        #[arg(long)]
        project_root: Option<String>,

        #[arg(long)]
        session_id_prefix: Option<String>,

        #[arg(long)]
        dry_run: bool,

        #[arg(long)]
        out_jsonl: Option<PathBuf>,
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

    Status {
        #[arg(long)]
        project_root: Option<String>,
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
