use super::common::ViewModeArgs;
use super::enums::{
    ExportFormat, ExportStrategy, InspectFormat, OutputFormat, PackTemplate, ProviderFilter,
    ProviderName, WatchFormat,
};
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Index and manage the session database")]
    Index {
        #[command(subcommand)]
        command: IndexCommand,
    },

    #[command(about = "View and analyze session data")]
    Session {
        #[command(subcommand)]
        command: SessionCommand,
    },

    #[command(about = "Configure log sources (Claude Code, Codex, Gemini)")]
    Provider {
        #[command(subcommand)]
        command: ProviderCommand,
    },

    #[command(about = "Diagnose and troubleshoot log parsing issues")]
    Doctor {
        #[command(subcommand)]
        command: DoctorCommand,
    },

    #[command(about = "List all indexed projects")]
    Project {
        #[command(subcommand)]
        command: ProjectCommand,
    },

    #[command(about = "Advanced analysis tools (grep, export, stats)")]
    Lab {
        #[command(subcommand)]
        command: LabCommand,
    },

    #[command(about = "List recent sessions (shorthand for 'session list')")]
    Sessions {
        #[arg(long)]
        project_hash: Option<String>,

        #[arg(long)]
        provider: Option<ProviderName>,

        #[arg(long, default_value = "50")]
        limit: usize,

        #[arg(long)]
        since: Option<String>,

        #[arg(long)]
        until: Option<String>,
    },

    #[command(about = "Package sessions for sharing or analysis")]
    Pack {
        #[arg(long, default_value = "compact")]
        template: PackTemplate,

        #[arg(long, default_value = "20")]
        limit: usize,
    },

    #[command(about = "Monitor live session updates in real-time")]
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
    },

    #[command(
        about = "Set up agtrace (detects providers, creates DB, scans logs)",
        long_about = "Initialize agtrace by detecting provider configurations, creating the database, and scanning logs.

This is typically the first command to run. It will:
  1. Auto-detect installed providers (Claude Code, Codex, Gemini)
  2. Create the SQLite index database
  3. Scan and index existing session logs

Use --refresh to force a re-scan even if recently indexed."
    )]
    Init {
        #[arg(long, help = "Force re-scan even if recently indexed")]
        refresh: bool,
    },
}

#[derive(Subcommand)]
pub enum IndexCommand {
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
Use filters to narrow down by provider, time range, or project."
    )]
    List {
        #[arg(long, help = "Filter by project hash")]
        project_hash: Option<String>,

        #[arg(long, help = "Filter by provider")]
        provider: Option<ProviderName>,

        #[arg(
            long,
            default_value = "50",
            help = "Maximum number of sessions to show"
        )]
        limit: usize,

        #[arg(long, help = "Show sessions after this timestamp")]
        since: Option<String>,

        #[arg(long, help = "Show sessions before this timestamp")]
        until: Option<String>,

        #[arg(long, help = "Skip automatic index refresh before listing")]
        no_auto_refresh: bool,

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

        #[arg(long, help = "Show detailed parsing errors")]
        verbose: bool,

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
Useful for validating log files before indexing."
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

This command scans event payloads from recent sessions and finds matches
containing the specified pattern. Useful for investigating tool call structures,
discovering actual argument formats, and debugging event data.

By default, shows a compact view with syntax highlighting. Use --json to see
the full JSON structure of matching events. Use --raw to see complete AgentEvent
including provider-specific metadata for normalization verification.",
        after_long_help = "EXAMPLES:
  # Find all ToolCall events containing 'Read'
  agtrace lab grep \"Read\" --limit 5

  # Search for file operations with JSON output
  agtrace lab grep \"file_path\" --json --limit 10

  # Find MCP-related calls from specific provider
  agtrace lab grep \"mcp\" --provider claude_code

  # Investigate write operations structure
  agtrace lab grep \"write_file\" --json

  # Verify normalization: compare raw provider schema with normalized content
  agtrace lab grep '\"name\":\"Read\"' --raw --limit 1

  # Debug MCP tool parsing
  agtrace lab grep '\"name\":\"mcp__o3__o3-search\"' --raw --limit 1

OUTPUT FORMAT (compact mode):
  ================================================================================
  Match #1 | Session: abc123def456... | Stream: Main
  Type: ToolCall
  Tool: Read
  Args: {\"file_path\":\"/path/to/project/src/main.rs\"}
  ================================================================================
  Match #2 | Session: abc123def456... | Stream: Main
  Type: Reasoning
  Text: Let me analyze the code structure...
  ================================================================================

OUTPUT FORMAT (JSON mode):
  ================================================================================
  Match #1 | Session: abc123def456... | Stream: Main
  {
    \"ToolCall\": {
      \"name\": \"Read\",
      \"arguments\": {
        \"file_path\": \"/path/to/project/src/main.rs\"
      }
    }
  }
  ================================================================================

OUTPUT FORMAT (raw mode):
  ================================================================================
  Match #1 | Session: d9967bbc | Stream: Main
  {
    \"id\": \"01a17cbe-fcc9-5670-9bb5-462918bbe3cb\",
    \"session_id\": \"d9967bbc-70cc-5624-bcd6-7e70824b84cb\",
    \"parent_id\": \"c4f8ccbf-f602-5d61-9c46-c594c6fb2aca\",
    \"timestamp\": \"2025-12-21T22:56:17.441Z\",
    \"stream_id\": { \"stream_type\": \"main\" },
    \"type\": \"tool_call\",
    \"content\": {
      \"name\": \"Read\",
      \"arguments\": { \"file_path\": \"/path/to/file.rs\" },
      \"provider_call_id\": \"toolu_017YauBeoeW2xdwPiMAebtsD\"
    },
    \"metadata\": {
      \"message\": {
        \"content\": [{
          \"id\": \"toolu_017YauBeoeW2xdwPiMAebtsD\",
          \"input\": { \"file_path\": \"/path/to/file.rs\" },
          \"name\": \"Read\",
          \"type\": \"tool_use\"
        }],
        \"model\": \"claude-sonnet-4-5-20250929\",
        ...
      }
    }
  }
  ================================================================================

NOTES:
  - Searches up to 1000 recent sessions by default
  - Pattern matching is case-sensitive substring search
  - Default limit is 50 matches
  - Use --provider to filter by provider (claude_code, codex, gemini)

RAW MODE RATIONALE:
  The --raw flag outputs complete AgentEvent including metadata. This enables:

  1. NORMALIZATION VERIFICATION
     Compare provider-specific schemas with normalized content:
     - Claude: metadata.message.content[].input vs content.arguments
     - Codex: metadata.payload.arguments (stringified) vs content.arguments (parsed)
     - Gemini: metadata.payload vs content

  2. DEBUGGING TOOL PARSING
     Inspect how normalize_tool_call() determines variants:
     - FileRead, FileEdit, FileWrite (file operations)
     - Execute (Bash, shell_command)
     - Search (Grep, WebSearch)
     - Mcp (mcp__server__tool format)
     - Generic (fallback)

  3. INVESTIGATION WITHOUT FILESYSTEM ACCESS
     Without --raw, verifying normalization requires:
     - Navigate ~/.claude/projects/*.jsonl or ~/.codex/sessions/*.jsonl
     - Manually correlate timestamps and event IDs
     - Parse provider-specific log formats

     With --raw, verification is streamlined:
     - Single command to inspect normalized + raw data
     - Session/stream context preserved
     - No filesystem traversal needed"
    )]
    Grep {
        #[arg(help = "String pattern to search for (e.g. 'write_file', 'mcp')")]
        pattern: String,

        #[arg(long, help = "Limit the number of matching events")]
        limit: Option<usize>,

        #[arg(long, help = "Filter by provider")]
        provider: Option<String>,

        #[arg(long, help = "Show raw JSON of the matching event")]
        json: bool,

        #[arg(long, help = "Show complete AgentEvent including metadata")]
        raw: bool,

        #[arg(
            long,
            help = "Use regex pattern matching instead of simple string contains"
        )]
        regex: bool,

        #[arg(
            long,
            value_name = "TYPE",
            help = "Filter by event type (ToolCall, ToolResult, User, Message, Reasoning, TokenUsage, Notification)"
        )]
        r#type: Option<String>,

        #[arg(long, help = "Filter by tool name (only for ToolCall events)")]
        tool: Option<String>,

        #[arg(long, help = "Case-insensitive search")]
        ignore_case: bool,
    },
}
