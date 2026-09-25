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
    about = "Monitor and analyze sessions from Claude Code and Codex.\nWorks like 'top' + 'tail -f' for AI agents — 100% local, no cloud."
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
        long = "project",
        alias = "project-root",
        global = true,
        help_heading = "Global Options",
        help = "Target project directory (defaults to current directory)"
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

    #[arg(
        long,
        global = true,
        hide_short_help = true,
        help_heading = "Global Options",
        help = "Include sessions from all git worktrees of the current repository"
    )]
    pub all_worktrees: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Parse a duration such as `90s`, `30m`, `2h` or `1d` (a bare number is minutes).
pub fn parse_since(s: &str) -> Result<std::time::Duration, String> {
    let s = s.trim();
    let (num, unit) = match s.find(|c: char| !c.is_ascii_digit()) {
        Some(i) => s.split_at(i),
        None => (s, "m"),
    };
    let n: u64 = num
        .parse()
        .map_err(|_| format!("invalid duration '{s}' (expected e.g. 30m, 2h, 1d)"))?;
    let secs = match unit {
        "s" => n,
        "m" => n * 60,
        "h" => n * 3_600,
        "d" => n * 86_400,
        _ => return Err(format!("invalid duration unit in '{s}' (use s, m, h or d)")),
    };
    if secs == 0 {
        return Err("duration must be positive".to_string());
    }
    Ok(std::time::Duration::from_secs(secs))
}

#[cfg(test)]
mod tests {
    use super::parse_since;
    use std::time::Duration;

    #[test]
    fn parses_since_units() {
        assert_eq!(parse_since("90s"), Ok(Duration::from_secs(90)));
        assert_eq!(parse_since("30m"), Ok(Duration::from_secs(1_800)));
        assert_eq!(parse_since("2h"), Ok(Duration::from_secs(7_200)));
        assert_eq!(parse_since("1d"), Ok(Duration::from_secs(86_400)));
        assert_eq!(parse_since("15"), Ok(Duration::from_secs(900)));
        assert!(parse_since("0h").is_err());
        assert!(parse_since("2w").is_err());
        assert!(parse_since("h").is_err());
    }
}
