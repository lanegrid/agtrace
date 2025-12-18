---
name: agtrace-cli-expert
description: agtrace CLI Expert - Deep knowledge of agtrace CLI structure, commands, handlers, and architecture. Automatically activates for CLI-related questions and modifications.
---

# agtrace CLI Expert

You are an expert in the agtrace CLI codebase. This skill provides comprehensive knowledge of the CLI structure, implementation patterns, and architecture.

## CLI Structure Overview

### Entry Point
- **crates/agtrace-cli/src/main.rs** - Main entry point using clap for argument parsing
- **crates/agtrace-cli/src/args.rs** - CLI structure definitions with all commands and options

### Available Commands

The CLI uses namespaced subcommands:

```
agtrace [OPTIONS] [COMMAND]

Commands:
  init       - Initialize agtrace configuration
  index      - Manage index database (update, rebuild, vacuum)
  session    - Manage sessions (list, show)
  provider   - Manage providers (list, detect, set, schema)
  doctor     - Diagnose provider configs (run, inspect, check)
  project    - List indexed projects
  lab        - Experimental features (export)
  sessions   - Alias for session list
  pack       - Analyze and pack sessions
  watch      - Watch for live session updates
```

### Global Options
- `--data-dir` (default: `~/.agtrace`) - Configuration and database directory
- `--format` (default: `plain`) - Output format (plain, json)
- `--log-level` (default: `info`) - Logging level (error, warn, info, debug, trace)
- `--project-root` (optional) - Override project root
- `--all-projects` (flag) - Include all projects

## Architecture Patterns

### Handler Pattern
Located in `crates/agtrace-cli/src/handlers/`:
- Each command has a dedicated handler (e.g., `session_show.rs`, `doctor_run.rs`)
- Handlers receive:
  1. Context/Database (for data access)
  2. Command-specific arguments
  3. `&dyn TraceView` (abstraction for rendering)
- Returns `Result<()>` for error handling

### View Abstraction
Located in `crates/agtrace-cli/src/ui/` and `crates/agtrace-cli/src/views/`:

**TraceView trait** - Unified interface combining four traits:
1. **SystemView** - System-level rendering
   - `render_guidance()` - Initial guidance
   - `render_provider_list()` - Provider configuration display
   - `render_index_event()` - Index operation progress
   - `render_init_event()` - Initialization progress
   - `render_corpus_overview()` - Database statistics
   - `render_project_list()` - Project listings

2. **SessionView** - Session-specific rendering
   - `render_session_list()` - Session listings
   - `render_session_compact()` - Compact view
   - `render_session_timeline()` - Timeline view
   - `render_session_events_json()` - JSON output
   - `render_pack_report()` - Pack analysis

3. **DiagnosticView** - Diagnostic rendering
   - `render_doctor_check()` - File format checking
   - `render_diagnose_results()` - Diagnostic results
   - `render_inspect()` - Raw file inspection
   - `render_provider_schema()` - Schema display

4. **WatchView** - Live streaming rendering
   - `render_watch_start()` - Start message
   - `on_watch_attached()` - Attach event
   - `on_watch_reaction()` - Stream event
   - `render_stream_update()` - Update display

### Execution Context
Located in `crates/agtrace-cli/src/context.rs`:
- Lazy-loaded database and configuration
- Provider resolution logic
- Project root management
- Methods:
  - `db()` - Lazy-load database
  - `config()` - Lazy-load configuration
  - `resolve_provider()` - Get specific provider
  - `resolve_providers()` - Get all enabled providers
  - `default_provider()` - Get first enabled provider

### Configuration System
Located in `crates/agtrace-cli/src/config.rs`:

```rust
pub struct ProviderConfig {
    pub enabled: bool,
    pub log_root: PathBuf,
    pub context_window_override: Option<u64>,
}

pub struct Config {
    pub providers: HashMap<String, ProviderConfig>,
}
```

- Stored in `~/.agtrace/config.toml`
- Auto-detected via `Config::detect_providers()`
- Methods: `load()`, `load_from()`, `save()`, `save_to()`

## Command Execution Flow

1. **Parse** - Clap parses CLI args into `Cli` struct
2. **Route** - Match on `Commands` enum in `commands.rs`
3. **Context** - Create `ExecutionContext` if needed
4. **Database** - Open database if needed
5. **Handle** - Call handler function with appropriate arguments
6. **Render** - Handler uses `TraceView` to render output

Default behavior: When no subcommand given, shows corpus overview or initial guidance.

## File Organization

```
crates/agtrace-cli/src/
├── main.rs              # Entry point
├── args.rs              # CLI definitions (clap derive)
├── commands.rs          # Command routing
├── context.rs           # Execution context
├── config.rs            # Configuration management
├── types.rs             # Enum types for CLI options
├── handlers/            # Command implementations
│   ├── mod.rs
│   ├── init.rs
│   ├── index.rs
│   ├── session_list.rs
│   ├── session_show.rs
│   ├── provider.rs
│   ├── provider_schema.rs
│   ├── doctor_run.rs
│   ├── doctor_inspect.rs
│   ├── doctor_check.rs
│   ├── project.rs
│   ├── lab_export.rs
│   ├── pack.rs
│   └── watch.rs
├── ui/                  # View traits
│   ├── mod.rs
│   ├── traits.rs
│   ├── console.rs
│   └── models.rs
├── views/               # Rendering implementations
│   ├── mod.rs
│   ├── init.rs
│   ├── doctor.rs
│   ├── pack.rs
│   ├── provider/
│   └── session/
├── display_model/       # Display data structures
│   ├── mod.rs
│   ├── init.rs
│   ├── doctor.rs
│   ├── provider.rs
│   └── session.rs
├── reactor.rs           # Re-exports from agtrace-runtime
├── reactors/            # Stream processing (watch)
│   ├── mod.rs
│   ├── safety_guard.rs
│   ├── stall_detector.rs
│   ├── token_usage_monitor.rs
│   └── tui_renderer.rs
├── streaming/           # Stream-related utilities
├── session_loader.rs    # Session data loading
├── token_limits.rs      # Token limit configuration
└── token_usage.rs       # Token usage tracking
```

## Design Principles

From `crates/agtrace-cli/src/lib.rs`:

- **Pointer-based architecture** - References log files rather than copying (handles schema evolution)
- **Schema-on-read** - Normalizes during viewing, not indexing (allows schema updates without re-indexing)
- **Fail-safe indexing** - Files registered even if parsing fails; use `doctor` to diagnose
- **Exact-match project isolation** - One directory = one project (use `--all-projects` for multiple)

## Key Dependencies

- **clap** (4.5) - CLI parsing with derive macros
- **serde/serde_json** - Serialization
- **agtrace-*** crates - Internal dependencies
- **owo-colors** - Terminal colors
- **comfy-table** - Table formatting
- **chrono** - Date/time handling
- **notify** - File system watching
- **anyhow** - Error handling

## When to Use This Skill

This skill should be activated when:
- Adding new commands or subcommands
- Modifying existing command handlers
- Working with view/rendering logic
- Debugging CLI argument parsing
- Understanding the CLI architecture
- Refactoring CLI components
- Adding new output formats
- Working with the execution context or configuration system

## Common Tasks

### Adding a New Command

1. Define command in `args.rs` (add to `Commands` enum)
2. Create handler in `handlers/` directory
3. Route command in `commands.rs`
4. Add view methods if needed in `ui/traits.rs`
5. Implement rendering in `views/` directory

### Adding a New Output Format

1. Add enum variant in `types.rs` (e.g., `OutputFormat`)
2. Update view trait methods to support new format
3. Implement rendering logic in appropriate view module
4. Update handlers to pass format information

### Modifying Execution Context

1. Update `ExecutionContext` struct in `context.rs`
2. Add lazy initialization if needed
3. Update `new()` or add getter methods
4. Update command handlers that need the new context

This architecture supports reliable log analysis even as provider schemas evolve, maintaining separation of concerns between parsing, routing, business logic, and presentation.
