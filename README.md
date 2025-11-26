# agtrace

**agtrace** is a Rust library and CLI tool that unifies session histories from AI coding agents (Claude Code, Codex, and others) into a standardized format for analysis and visualization.

Part of the [lanegrid](https://github.com/lanegrid/lanegrid) project, but designed to be useful as a standalone tool.

## Features

- **Unified Format**: One data model for all agents
- **Zero Configuration**: Auto-detect agent data from standard locations
- **Lossless Parsing**: Preserve all meaningful information from source logs
- **Library First**: Usable as a Rust crate, CLI is a thin wrapper
- **Extensible**: Easy to add new agent parsers
- **Privacy Aware**: All data stays local, no network calls
- **No Persistence**: Reads directly from agent directories on each invocation (source of truth)

## Installation

```bash
cargo install agtrace
```

Or build from source:

```bash
git clone https://github.com/lanegrid/agtrace
cd agtrace
cargo build --release
```

## CLI Usage

### List Executions

```bash
# List all executions from all agents
agtrace list

# Filter by agent
agtrace list --agent claude-code
agtrace list --agent codex

# Read from custom location
agtrace list --agent claude-code --path /custom/path

# Filter by project
agtrace list --project ./my-repo

# Filter by date
agtrace list --since 2025-01-01

# Limit results
agtrace list --limit 20
```

### Show Execution Details

```bash
# Show basic details (requires agent type and execution ID)
agtrace show claude-code <execution-id>
agtrace show codex <session-id>

# Include event timeline
agtrace show claude-code <execution-id> --events

# Output as JSON
agtrace show claude-code <execution-id> --json

# Read from custom location
agtrace show claude-code <execution-id> --path /custom/path
```

### Statistics

```bash
# Overall statistics
agtrace stats

# Filter by agent
agtrace stats --agent claude-code

# Grouped by agent
agtrace stats --by-agent

# Grouped by project
agtrace stats --by-project

# Grouped by day
agtrace stats --by-day

# Read from custom location
agtrace stats --agent claude-code --path /custom/path
```

### Export

```bash
# Export single execution
agtrace export claude-code <execution-id> --format json

# Export all executions as JSON
agtrace export --all --format json

# Export all executions as JSONL
agtrace export --all --format jsonl

# Export all from specific agent
agtrace export --all --agent claude-code --format jsonl
```

## Library Usage

```rust
use agtrace::{parser, storage, Execution, Event};

// Parse Claude Code sessions from default directory (~/.claude)
let executions = parser::claude_code::parse_default_dir()?;

// Parse from custom directory
let executions = parser::claude_code::parse_dir("/custom/path")?;

// Parse Codex sessions
let executions = parser::codex::parse_default_dir()?;

// List all executions from all agents
let all_executions = storage::list_all_executions()?;

// Find a specific execution by ID
let execution = storage::find_execution("session-id")?;

// Access execution data
for exec in all_executions {
    println!("Session: {}", exec.id);
    println!("Project: {:?}", exec.project_path);
    println!("Duration: {:?}s", exec.metrics.duration_seconds);
    println!("Tool calls: {}", exec.metrics.tool_call_count);

    for event in &exec.events {
        match event {
            Event::ToolCall { name, timestamp, .. } => {
                println!("  {} at {}", name, timestamp);
            }
            _ => {}
        }
    }
}
```

## Data Model

The core data model consists of:

- **Execution**: A single agent session with metadata, events, and metrics
- **Event**: User messages, assistant messages, thinking blocks, tool calls, and tool results
- **Agent**: Which agent produced the execution (Claude Code, Codex, etc.)
- **ExecutionMetrics**: Aggregated statistics about the session

See the [design document](DESIGN_DOCUMENT.md) for full details.

## Architecture

agtrace reads directly from agent data directories on each invocation:

- **Claude Code**: Reads from `~/.claude/projects/`
- **Codex**: Reads from `~/.codex/sessions/`

**Why no persistence?**
- Source directories are the single source of truth
- No sync issues between original and cached data
- Simpler implementation
- Consumers (like lanegrid) can implement their own persistence if needed

## Supported Agents

- **Claude Code**: Parses from `~/.claude/projects/`
- **Codex**: Parses from `~/.codex/sessions/`

More agents coming soon!

## Development

```bash
# Build
cargo build

# Run tests
cargo test

# Run CLI
cargo run -- --help
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

MIT OR Apache-2.0

## Related Projects

- [lanegrid](https://github.com/lanegrid/lanegrid) - The main project that uses agtrace
