// NOTE: Command Organization Rationale
//
// Why namespaced subcommands (not flat)?
// - Flat command structures become unwieldy past ~10 commands
// - Namespaces (index, session, provider, doctor, lab, project) group related operations
// - Improves --help discoverability and conceptual clarity
// - Example: `session show` vs `session list` vs flat `show-session` and `list-sessions`

mod commands;
mod common;
mod enums;
pub mod hints;

pub use commands::*;
pub use common::*;
pub use enums::*;

use clap::Parser;

#[derive(Parser)]
#[command(name = "agtrace")]
#[command(
    about = "Monitor and analyze sessions from Claude Code, Codex, and Gemini.\nWorks like 'top' + 'tail -f' for AI agents — 100% local, no cloud."
)]
#[command(after_help = "Quick Start:\n  \
                        agtrace init      # Run once to set up\n  \
                        agtrace watch     # Monitor sessions in real-time\n\n\
                        Learn more: https://github.com/lanegrid/agtrace")]
#[command(version)]
pub struct Cli {
    #[arg(
        long,
        global = true,
        hide_short_help = true,
        help_heading = "Global Options",
        help = "Directory for agtrace database and configuration (defaults to system data directory, or AGTRACE_PATH env var)"
    )]
    pub data_dir: Option<String>,

    #[arg(
        long,
        default_value = "plain",
        global = true,
        help_heading = "Global Options",
        help = "Output format for command results"
    )]
    pub format: OutputFormat,

    #[arg(
        long,
        default_value = "info",
        global = true,
        hide_short_help = true,
        help_heading = "Global Options",
        help = "Logging verbosity level"
    )]
    pub log_level: LogLevel,

    #[arg(
        long,
        global = true,
        help_heading = "Global Options",
        help = "Override project root directory (defaults to current directory)"
    )]
    pub project_root: Option<String>,

    #[arg(
        long,
        global = true,
        hide_short_help = true,
        help_heading = "Global Options",
        help = "Operate on all indexed projects instead of current project only"
    )]
    pub all_projects: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}
