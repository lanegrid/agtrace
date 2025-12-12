# agtrace

Normalize and analyze agent behavior logs from Claude Code, Codex, and Gemini CLI.

## Documentation

- [CLI Specification](docs/agtrace_cli_spec.md) - Complete command reference
- [Troubleshooting Schema Issues](docs/troubleshooting_schema_issues.md) - Guide for fixing schema compatibility
- [Database Schema](docs/database_schema.md) - SQLite database structure
- [Agent Event Schema](docs/agent_event_schema_v1.md) - Normalized event format

## Quick Start

```bash
# Auto-detect providers and create configuration
agtrace provider detect

# Scan and index sessions for current project
agtrace index update

# List recent sessions
agtrace session list --limit 10

# View a session timeline
agtrace session show <session_id>
```

## Migration Note (Command Structure Update)

As of the latest version, agtrace commands have been reorganized into namespaces for better discoverability:

### New Command Structure

- `agtrace index {update|rebuild|vacuum}` - Database operations
- `agtrace session {list|show}` - Session management
- `agtrace provider {list|detect|set|schema}` - Provider configuration
- `agtrace doctor {run|inspect|check}` - Diagnostics
- `agtrace project {list}` - Project information
- `agtrace lab {analyze|export}` - Experimental features

### Legacy Commands (Deprecated)

The old flat command structure is still supported but deprecated. You'll see warnings when using these:

| Old Command | New Command | Notes |
|-------------|-------------|-------|
| `scan` | `index update` | Incremental scan |
| `scan --force` | `index rebuild` | Force full rescan |
| `list` | `session list` | List sessions |
| `view` / `show` | `session show` | View session details |
| `providers` | `provider ...` | Provider commands |
| `schema <p>` | `provider schema <p>` | View provider schema |
| `diagnose` | `doctor run` | Diagnose issues |
| `inspect <f>` | `doctor inspect <f>` | Inspect log files |
| `validate <f>` | `doctor check <f>` | Validate files |
| `analyze` | `lab analyze` | Analyze sessions |
| `export` | `lab export` | Export sessions |

### Why the Change?

The new hierarchical structure:
- Makes related commands easier to discover (`--help` shows organized groups)
- Allows future expansion without cluttering the top level
- Follows common CLI patterns (like `git`, `docker`, `kubectl`)
- Keeps backwards compatibility through deprecation warnings

### Migration Timeline

- **Now**: Both old and new commands work
- **Warnings**: Legacy commands show deprecation messages to stderr
- **Future**: Legacy commands may be removed in a major version update

