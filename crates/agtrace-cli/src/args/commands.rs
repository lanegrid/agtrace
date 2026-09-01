use super::common::ViewModeArgs;
use super::enums::{
    DumpFormat, ExportFormat, ExportStrategy, InspectFormat, OutputFormat, PackTemplate,
    ProviderFilter, ProviderName, WatchFormat,
};
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum Commands {
    // Core Commands (no heading - highest priority)
    #[command(
        about = "Initialize agtrace workspace (run once to get started)",
        long_about = "Initialize agtrace — run this once to get started.

This command will:
  • Auto-detect installed providers (Claude Code, Codex, Gemini)
  • Create the local database (system data directory, e.g., ~/Library/Application Support/agtrace on macOS)
  • Scan and index existing session logs

After running 'init', use 'agtrace watch' to monitor sessions in real-time.

Use --refresh to force a re-scan of all logs."
    )]
    Init {
        #[arg(long, help = "Force re-scan even if recently indexed")]
        refresh: bool,
    },

    #[command(about = "Monitor live agent sessions in real-time (TUI dashboard)")]
    Watch {
        #[arg(long, help = "Filter by provider")]
        provider: Option<ProviderName>,

        #[arg(
            long,
            help = "Explicit session ID or file path to watch (bypasses liveness detection)"
        )]
        id: Option<String>,

        #[arg(
            long,
            default_value = "tui",
            help = "Display mode: tui (interactive) or console (streaming text)"
        )]
        mode: WatchFormat,

        #[arg(
            long,
            help = "Enable debug mode (Ctrl+T: add turn, Ctrl+D: add 25 turns)"
        )]
        debug: bool,
    },

    #[command(about = "Enable agent self-reflection via MCP (Model Context Protocol)")]
    Mcp {
        #[command(subcommand)]
        command: McpCommand,
    },

    // Session & Provider Management
    #[command(
        next_help_heading = "Session & Provider Management",
        about = "View and analyze past sessions"
    )]
    Session {
        #[command(subcommand)]
        command: SessionCommand,
    },

    #[command(about = "Configure log sources (Claude Code, Codex, Gemini)")]
    Provider {
        #[command(subcommand)]
        command: ProviderCommand,
    },

    // Additional Commands
    #[command(
        next_help_heading = "Additional Commands",
        about = "List recent sessions (shorthand for 'session list')"
    )]
    Sessions {
        #[arg(long)]
        project_hash: Option<String>,

        #[arg(long)]
        provider: Option<ProviderName>,

        #[arg(long, default_value = "10")]
        limit: usize,

        #[arg(long)]
        since: Option<String>,

        #[arg(long)]
        until: Option<String>,

        #[arg(long, help = "Include child sessions (subagents) in the list")]
        all: bool,
    },

    #[command(about = "Advanced analysis tools (grep, export, stats)")]
    Lab {
        #[command(subcommand)]
        command: LabCommand,
    },

    #[command(about = "Manage the session index (update, rebuild, vacuum)")]
    Index {
        #[command(subcommand)]
        command: IndexCommand,
    },

    #[command(about = "List all indexed projects")]
    Project {
        #[command(subcommand)]
        command: ProjectCommand,
    },

    #[command(about = "Diagnose and troubleshoot log parsing issues")]
    Doctor {
        #[command(subcommand)]
        command: DoctorCommand,
    },

    #[command(about = "Package sessions for sharing or analysis")]
    Pack {
        #[arg(long, default_value = "compact")]
        template: PackTemplate,

        #[arg(long, default_value = "20")]
        limit: usize,
    },

    #[command(
        about = "Run a simulated live demo of the TUI dashboard",
        long_about = "Start a simulated session showing how agtrace monitors an AI agent in real-time.

This allows you to experience the TUI dashboard without needing active agent logs.
It simulates a refactoring session to demonstrate context window tracking and event flow.

Perfect for understanding agtrace's capabilities before setting up your own logs."
    )]
    Demo {
        #[arg(
            long,
            default_value = "normal",
            help = "Simulation speed: slow, normal, or fast"
        )]
        speed: String,
    },
}

#[derive(Subcommand)]
pub enum McpCommand {
    #[command(
        about = "Start MCP server for agent self-reflection",
        long_about = "Start the Model Context Protocol (MCP) server over stdio.

This enables AI coding assistants (Claude Code, Codex, Gemini CLI, Claude Desktop) to query
their own execution history, analyze failures, search event payloads, and debug behavior.

The server exposes these tools:
  • list_sessions: Browse session history with cursor-based pagination
  • get_project_info: List all indexed projects with metadata
  • analyze_session: Run diagnostic analysis (failures, loops, issues)
  • search_events: Search events and return navigation coordinates
  • list_turns: List turns with metadata only (no payload content)
  • get_turns: Get details for specific turns with safety valves

Configure in claude_desktop_config.json to use with Claude Desktop."
    )]
    Serve,

    #[command(
        about = "Test MCP server response sizes",
        long_about = "Test the MCP server by sending requests and measuring response sizes.

This tool helps identify endpoints that return too much data for AI agent consumption.
It spawns an MCP server instance and tests each endpoint with various parameters."
    )]
    Test {
        #[arg(long, help = "Show detailed response content")]
        verbose: bool,
    },
}

#[derive(Subcommand)]
pub enum IndexCommand {
    #[command(
        about = "Show database location and index statistics",
        long_about = "Display information about the agtrace database and index.

Shows:
  • Data directory path (where database is stored)
  • Database file path
  • Index statistics (sessions, files, etc.)"
    )]
    Info,

    #[command(
        about = "Scan for new sessions and add them to the index",
        long_about = "Incrementally update the index by scanning for new session logs.

This command only processes files that haven't been indexed yet, making it fast for regular updates.
Use this after AI agent sessions to make them searchable."
    )]
    Update {
        #[arg(long, default_value = "all", help = "Filter by provider")]
        provider: ProviderFilter,

        #[command(flatten)]
        view_mode: ViewModeArgs,
    },

    #[command(
        about = "Clear and rebuild the entire index from scratch",
        long_about = "Rebuild the entire index by re-scanning all log files.

This is useful when:
  - You suspect the index is corrupted
  - Provider log formats have changed
  - You want to force re-processing of all sessions

Warning: This clears all existing index data."
    )]
    Rebuild {
        #[arg(long, default_value = "all", help = "Filter by provider")]
        provider: ProviderFilter,

        #[command(flatten)]
        view_mode: ViewModeArgs,
    },

    #[command(about = "Optimize database performance and reclaim disk space")]
    Vacuum {
        #[command(flatten)]
        view_mode: ViewModeArgs,
    },
}

#[derive(Subcommand)]
pub enum SessionCommand {
    #[command(
        about = "Browse recent sessions with optional filtering",
        long_about = "List recent AI agent sessions from the index.

Sessions are displayed with timestamps, snippets, and metadata.
Use filters to narrow down by provider, time range, or project.",
        after_long_help = "EXAMPLES:
  # List 10 most recent sessions
  agtrace session list --limit 10

  # Filter by provider
  agtrace session list --provider claude_code

  # Show sessions from the last 2 hours (ISO 8601 format)
  agtrace session list --since 2025-01-02T10:00:00Z

  # Show sessions from a specific time range
  agtrace session list --since 2025-01-01T00:00:00Z --until 2025-01-02T00:00:00Z

  # JSON output for scripting
  agtrace session list --format json --limit 5"
    )]
    List {
        #[arg(long, help = "Filter by project hash")]
        project_hash: Option<String>,

        #[arg(long, help = "Filter by provider")]
        provider: Option<ProviderName>,

        #[arg(
            long,
            default_value = "10",
            help = "Maximum number of sessions to show"
        )]
        limit: usize,

        #[arg(long, help = "Show sessions after this timestamp")]
        since: Option<String>,

        #[arg(long, help = "Show sessions before this timestamp")]
        until: Option<String>,

        #[arg(long, help = "Skip automatic index refresh before listing")]
        no_auto_refresh: bool,

        #[arg(long, help = "Include child sessions (subagents) in the list")]
        all: bool,

        #[arg(long, default_value = "plain", help = "Output format")]
        format: OutputFormat,

        #[command(flatten)]
        view_mode: ViewModeArgs,
    },

    #[command(
        about = "Show detailed analysis of a specific session",
        long_about = "Display comprehensive analysis of a single session including:
  - Context window usage and token statistics
  - Turn-by-turn conversation flow
  - Tool calls and reasoning traces
  - Model and provider information

Use this to deep-dive into session behavior and performance."
    )]
    Show {
        #[arg(help = "Session ID (short or full hash)")]
        session_id: String,

        #[arg(long, default_value = "plain", help = "Output format")]
        format: OutputFormat,

        #[command(flatten)]
        view_mode: ViewModeArgs,
    },

    #[command(
        about = "Export session events as structured data for analysis",
        long_about = "Dump session events in JSONL format for universal debugging and analysis.

Output format (default - normalized events):
  {\"seq\":0,\"timestamp\":\"...\",\"type\":\"User\",\"content\":{...},\"turn_idx\":0}
  {\"seq\":1,\"timestamp\":\"...\",\"type\":\"TokenUsage\",\"content\":{...},\"turn_idx\":0,\"step_idx\":0}
  {\"seq\":2,\"timestamp\":\"...\",\"type\":\"ToolCall\",\"content\":{...},\"turn_idx\":0,\"step_idx\":0}

With --raw flag (includes provider metadata for normalization verification):
  {\"seq\":0,\"type\":\"TokenUsage\",\"normalized\":{...},\"provider\":{\"name\":\"claude_code\",\"raw_payload\":{...}}}

Use cases:
  - Token monotonicity: dump <id> | jq -s 'map(select(.type == \"TokenUsage\")) | map(.content.total)'
  - Tool patterns: dump <id> | jq -r 'select(.type == \"ToolCall\") | .content.name' | sort | uniq -c
  - Turn boundaries: dump <id> | jq -r 'select(.type == \"User\") | .timestamp'
  - Verify normalization: dump <id> --raw | jq 'select(.normalized != .provider.raw_payload)'"
    )]
    Dump {
        #[arg(help = "Session ID (short or full hash)")]
        session_id: String,

        #[arg(
            long,
            help = "Include provider raw metadata for normalization verification"
        )]
        raw: bool,

        #[arg(
            long,
            default_value = "jsonl",
            value_name = "FORMAT",
            help = "Output format: jsonl (one event per line) or json (array)"
        )]
        output: DumpFormat,

        #[command(flatten)]
        view_mode: ViewModeArgs,
    },
}

#[derive(Subcommand)]
pub enum ProviderCommand {
    #[command(about = "Show all configured log sources and their status")]
    List {
        #[command(flatten)]
        view_mode: ViewModeArgs,
    },
    #[command(
        about = "Auto-detect installed AI tools and configure them",
        long_about = "Scan the system for installed AI agent tools and automatically configure their log paths.

Supported providers:
  - Claude Code (~/.claude/projects)
  - Codex (~/.codex/sessions)
  - Gemini (~/.gemini/tmp)

Detected providers are saved to the configuration file."
    )]
    Detect {
        #[command(flatten)]
        view_mode: ViewModeArgs,
    },
    #[command(
        about = "Manually configure a log source",
        long_about = "Manually add or update a provider configuration.

Use this when:
  - Auto-detection fails
  - Logs are in a non-standard location
  - You want to enable/disable a specific provider"
    )]
    Set {
        #[arg(help = "Provider name (claude_code, codex, gemini)")]
        provider: String,

        #[arg(long, help = "Path to the provider's log directory")]
        log_root: PathBuf,

        #[arg(long, help = "Enable this provider for indexing")]
        enable: bool,

        #[arg(long, help = "Disable this provider from indexing")]
        disable: bool,

        #[command(flatten)]
        view_mode: ViewModeArgs,
    },
}

#[derive(Subcommand)]
pub enum DoctorCommand {
    #[command(
        about = "Scan all log files and report parsing errors",
        long_about = "Run diagnostics on all configured provider log files.

This command:
  - Scans all log files from enabled providers
  - Attempts to parse each file
  - Reports files that fail to parse
  - Provides error details for troubleshooting

Use this when sessions aren't appearing in the index."
    )]
    Run {
        #[arg(long, default_value = "all", help = "Filter by provider")]
        provider: ProviderFilter,

        #[command(flatten)]
        view_mode: ViewModeArgs,
    },

    #[command(
        about = "View raw contents of a log file",
        long_about = "Display the raw contents of a log file for manual inspection.

Useful for debugging parsing issues or understanding log file structure."
    )]
    Inspect {
        #[arg(help = "Path to the log file")]
        file_path: String,

        #[arg(long, default_value = "50", help = "Number of lines to display")]
        lines: usize,

        #[arg(long, default_value = "raw", help = "Display format")]
        format: InspectFormat,

        #[command(flatten)]
        view_mode: ViewModeArgs,
    },

    #[command(
        about = "Validate if a specific log file can be parsed",
        long_about = "Test if a specific log file can be successfully parsed.

Returns success/failure status and event count.
Useful for validating log files before indexing.",
        after_long_help = "EXAMPLES:
  # Check if a log file can be parsed (auto-detect provider)
  agtrace doctor check ~/.claude/projects/my-project/session.jsonl

  # Explicitly specify provider
  agtrace doctor check --provider claude_code ~/.claude/projects/my-project/session.jsonl

  # Check multiple files
  agtrace doctor check ~/.codex/sessions/*.jsonl"
    )]
    Check {
        #[arg(help = "Path to the log file")]
        file_path: String,

        #[arg(long, help = "Explicitly specify provider (auto-detects if omitted)")]
        provider: Option<ProviderName>,

        #[command(flatten)]
        view_mode: ViewModeArgs,
    },
}

#[derive(Subcommand)]
pub enum ProjectCommand {
    #[command(about = "List all indexed projects")]
    List {
        #[arg(long)]
        project_root: Option<String>,

        #[command(flatten)]
        view_mode: ViewModeArgs,
    },
}

#[derive(Subcommand)]
pub enum LabCommand {
    #[command(
        about = "Export session data to various formats",
        after_long_help = "EXAMPLES:
  # Export to default file (session_abc123de.jsonl)
  agtrace lab export abc123def

  # Export to a specific file
  agtrace lab export abc123def --output session.jsonl

  # Export as plain text
  agtrace lab export abc123def --export-format text --output session.txt

  # Export with different strategies (filtering events)
  agtrace lab export abc123def --strategy clean --output session.jsonl"
    )]
    Export {
        #[arg(help = "Session ID (short or full hash)")]
        session_id: String,

        #[arg(long, help = "Output file path (defaults to session_{id}.jsonl)")]
        output: Option<PathBuf>,

        #[arg(long, default_value = "jsonl", help = "Export file format")]
        export_format: ExportFormat,

        #[arg(long, default_value = "raw", help = "Export strategy")]
        strategy: ExportStrategy,
    },

    #[command(about = "Analyze ToolCall statistics across sessions")]
    Stats {
        #[arg(long)]
        limit: Option<usize>,

        #[arg(long)]
        provider: Option<String>,
    },

    #[command(
        about = "Search for patterns in event payloads across sessions",
        long_about = "Search for patterns in event payloads across sessions.

Supports substring, glob (* ?), and regex (--regex) patterns.
Glob patterns are auto-detected when * or ? is present.",
        after_long_help = "EXAMPLES:
  agtrace lab grep \"Read\"              # Substring search
  agtrace lab grep \"*mcp__*\"           # Glob pattern
  agtrace lab grep \"file.*\" --regex    # Regex pattern
  agtrace lab grep \"Read\" --json       # JSON output"
    )]
    Grep {
        #[arg(help = "Pattern to search (glob * ? auto-detected)")]
        pattern: String,

        #[arg(long, help = "Max matches [default: 10]")]
        limit: Option<usize>,

        #[arg(long, help = "Filter by provider (claude_code, codex, gemini)")]
        provider: Option<String>,

        #[arg(long, help = "Show JSON output")]
        json: bool,

        #[arg(long, help = "Show complete AgentEvent with metadata")]
        raw: bool,

        #[arg(long, help = "Use regex instead of substring/glob")]
        regex: bool,

        #[arg(long, value_name = "TYPE", help = "Filter by event type")]
        r#type: Option<String>,

        #[arg(long, help = "Filter by tool name (ToolCall only)")]
        tool: Option<String>,

        #[arg(long, short = 'i', help = "Case-insensitive search")]
        ignore_case: bool,
    },
}
