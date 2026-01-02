# Architecture

agtrace is designed as a **layered platform**, separating the core domain logic from the presentation layer. It follows **pointer-based indexing** and **schema-on-read** principles to ensure resilient, low-overhead observability for AI coding agent sessions.

## Layered Platform Design

agtrace is architected as a platform with clear separation of concerns:

```
┌──────────────────────────────────────────┐
│         Applications Layer               │
│  ┌────────────┐  ┌──────────────┐       │
│  │ agtrace-cli│  │ vital-checker│  ...  │
│  │    (TUI)   │  │   (Monitor)  │       │
│  └────────────┘  └──────────────┘       │
└─────────────┬────────────────────────────┘
              │
              ▼
┌──────────────────────────────────────────┐
│        Public SDK Layer                  │
│  ┌────────────────────────────────────┐  │
│  │        agtrace-sdk                 │  │
│  │  (Stable, High-level API)          │  │
│  └────────────────────────────────────┘  │
└─────────────┬────────────────────────────┘
              │
              ▼
┌──────────────────────────────────────────┐
│        Core Logic Layer (Internal)       │
│  ┌──────────┐ ┌─────────┐ ┌───────────┐ │
│  │ runtime  │ │ engine  │ │   index   │ │
│  └──────────┘ └─────────┘ └───────────┘ │
└─────────────┬────────────────────────────┘
              │
              ▼
┌──────────────────────────────────────────┐
│        Adapter Layer                     │
│  ┌────────────────────────────────────┐  │
│  │      agtrace-providers             │  │
│  │  (Normalize Claude, Codex, Gemini) │  │
│  └────────────────────────────────────┘  │
└──────────────────────────────────────────┘
```

**Key insight**: The CLI is just one consumer of the SDK. Developers can build custom monitoring tools, dashboards, or IDE integrations using the same stable API.

## Core Principles

### 1. No Data Duplication

agtrace does not copy your massive log files. It indexes metadata and points to the original logs.

**Why this matters:**
- Agent logs can grow to hundreds of megabytes per session
- Duplicating this data would waste disk space and slow down operations
- The original logs are the source of truth

**Implementation:**
- SQLite database stores only metadata (session IDs, timestamps, file paths)
- Original JSONL log files remain in their provider-specific locations
- When you query a session, agtrace reads from the original log files on demand

### 2. Resilient to Schema Drift

Provider log schemas change frequently. agtrace parses logs at read time, so schema updates are less likely to corrupt or invalidate historical indexes.

**Why this matters:**
- AI coding agents evolve rapidly, and their log formats change
- Parsing at write time locks you into a specific schema version
- Historical sessions become unreadable when schemas change

**Implementation (Schema-on-Read):**
- Raw log files are never modified
- Parsing logic is applied when you run commands like `session show` or `lab grep`
- If a schema changes, you can update the parser without losing historical data
- The database can be rebuilt from raw logs at any time

### 3. Project Isolation

Sessions are scoped by cwd/project boundaries and grouped by a project root hash to keep workspaces clean and prevent cross-project mixing.

**Why this matters:**
- Different projects should have isolated session histories
- Multi-repository workflows need clear boundaries
- You shouldn't see sessions from other projects in your current workspace

**Implementation:**
- Project root is determined by the current working directory (cwd)
- A hash of the project path serves as the project identifier
- Sessions are tagged with this project hash at indexing time
- Queries filter by the current project's hash automatically

## Data Flow

```
┌─────────────────┐
│  Provider Logs  │ (Claude Code, Codex, Gemini)
│  ~/.claude/     │
│  ~/.codex/      │
│  ~/.gemini/     │
└────────┬────────┘
         │
         │ Discovery
         ▼
┌─────────────────┐
│  agtrace-index  │ (Metadata only: session IDs, paths, timestamps)
│  System data    │ (e.g., ~/Library/Application Support/agtrace)
│  agtrace.db     │
└────────┬────────┘
         │
         │ Query
         ▼
┌─────────────────┐
│ agtrace-engine  │ (Parse on demand, reconstruct sessions)
└────────┬────────┘
         │
         │ Present
         ▼
┌─────────────────┐
│  agtrace-cli    │ (TUI, JSON output)
└─────────────────┘
```

## Key Components

### agtrace-sdk (Public Facade)

The unified entry point for building observability tools. Provides a stable, high-level API.

**Responsibilities:**
- Abstract internal complexity (runtime, indexing, providers)
- Provide clean API for watching, querying, and analyzing sessions
- Enable third-party tool development (vital-checkers, IDE plugins, dashboards)
- Maintain API stability across internal refactors

**Example Usage:**
```rust
let client = Client::connect_default().await?;
let stream = client.watch().all_providers().start()?;
```

### agtrace-providers (Adapter Layer)

Normalizes diverse provider log formats into a unified `AgentEvent` model.

**Responsibilities:**
- Discover log files in provider-specific locations
- Parse JSONL streams into structured events
- Map provider-specific tool names to standard types
- Handle schema variations across different provider versions

### agtrace-index (Storage Layer)

Maintains a lightweight SQLite database for fast session lookup.

**Responsibilities:**
- Store session metadata (ID, start/end times, project hash, file paths)
- Enable fast queries like "show me all sessions for this project"
- Track which log files belong to which sessions
- Never duplicate log content

### agtrace-engine (Domain Logic)

Reconstructs session timelines and calculates metrics.

**Responsibilities:**
- Parse event streams into turns and steps
- Track context window usage over time
- Calculate token counts and costs
- Extract reasoning chains and tool usage patterns
- Provide diagnostic lenses (Failures, Loops, Bottlenecks)

### agtrace-cli (Reference Application)

The official CLI application built on top of `agtrace-sdk`. Demonstrates best practices.

**Responsibilities:**
- Live `watch` TUI for active sessions
- `session` commands for historical inspection
- `lab` commands for advanced queries
- JSON export for programmatic use

## Storage Layout

```
System data directory/agtrace/  # e.g., ~/Library/Application Support/agtrace on macOS
├── agtrace.db               # SQLite metadata index
└── config.toml              # User configuration

~/.claude/              # Example: Claude Code logs (not modified by agtrace)
└── sessions/
    └── <session-id>/
        └── events.jsonl

~/.codex/               # Example: Codex logs (not modified by agtrace)
└── sessions/
    └── <session-id>/
        └── stream.jsonl
```

## Design Trade-offs

| Decision | Trade-off | Rationale |
|----------|-----------|-----------|
| Pointer-based index | Query latency vs storage | Log files can be gigabytes; duplication is prohibitive |
| Schema-on-read | Parse cost vs resilience | Provider schemas change frequently; reindexing is cheaper than data loss |
| CWD-based scoping | Simplicity vs flexibility | Most workflows are single-project; multi-project users can `cd` |
| SQLite for metadata | Deployment simplicity vs scale | Targets individual developers, not production observability |

## Future Directions

- **Incremental indexing** to reduce startup time for large log directories
- **Compaction detection** to alert when context window pressure is high
- **Cross-session comparison** to identify patterns across multiple runs
- **Export to observability platforms** for teams that want centralized dashboards
