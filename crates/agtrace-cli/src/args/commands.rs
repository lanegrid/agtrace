use super::common::ViewModeArgs;
use super::enums::{
    ExportFormat, ExportStrategy, InspectFormat, OutputFormat, PackTemplate, ProviderFilter,
    ProviderName, WatchFormat,
};
use clap::Subcommand;
use std::path::PathBuf;

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

        #[arg(
            long,
            default_value = "tui",
            help = "Display mode: tui (interactive) or console (streaming text)"
        )]
        mode: WatchFormat,
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

        #[command(flatten)]
        view_mode: ViewModeArgs,
    },

    #[command(about = "Rebuild the entire index from scratch")]
    Rebuild {
        #[arg(long, default_value = "all")]
        provider: ProviderFilter,

        #[command(flatten)]
        view_mode: ViewModeArgs,
    },

    #[command(about = "Optimize database by reclaiming unused space")]
    Vacuum {
        #[command(flatten)]
        view_mode: ViewModeArgs,
    },
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

        #[arg(long, default_value = "plain", help = "Output format")]
        format: OutputFormat,

        #[command(flatten)]
        view_mode: ViewModeArgs,
    },

    #[command(about = "Display session analysis with context usage and turn metrics")]
    Show {
        session_id: String,

        #[arg(long, default_value = "plain", help = "Output format")]
        format: OutputFormat,

        #[command(flatten)]
        view_mode: ViewModeArgs,
    },
}

#[derive(Subcommand)]
pub enum ProviderCommand {
    #[command(about = "Show configured providers")]
    List {
        #[command(flatten)]
        view_mode: ViewModeArgs,
    },
    #[command(about = "Auto-detect and configure providers")]
    Detect {
        #[command(flatten)]
        view_mode: ViewModeArgs,
    },
    #[command(about = "Configure a provider")]
    Set {
        provider: String,

        #[arg(long)]
        log_root: PathBuf,

        #[arg(long)]
        enable: bool,

        #[arg(long)]
        disable: bool,

        #[command(flatten)]
        view_mode: ViewModeArgs,
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

        #[command(flatten)]
        view_mode: ViewModeArgs,
    },

    #[command(about = "Inspect raw log file contents")]
    Inspect {
        file_path: String,

        #[arg(long, default_value = "50")]
        lines: usize,

        #[arg(long, default_value = "raw")]
        format: InspectFormat,

        #[command(flatten)]
        view_mode: ViewModeArgs,
    },

    #[command(about = "Check if a log file can be parsed")]
    Check {
        file_path: String,

        #[arg(long)]
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
        source: Option<String>,
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
  agtrace lab grep \"mcp\" --source claude_code

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
  - Use --source to filter by provider (claude_code, codex, gemini)

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
     - Navigate ~/.claude/sessions/*.jsonl or ~/.codex/sessions/*.jsonl
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
        source: Option<String>,

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
