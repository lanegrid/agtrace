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
│  │  (Normalize Claude Code, Codex)    │  │
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

agtrace has two paths over the same provider decoders: a **live path** for `watch`, and a
**history path** for `session`, `lab` and MCP.

```
                    Provider logs (never modified)
        ~/.claude/projects, teams/, sessions/      ~/.codex/sessions
                                 │
                                 ▼
┌──────────────────────────────────────────────────────────────────────┐
│ agtrace-providers                                                    │
│  Provider::discover / read_header  → FileHeader (AgentRef: id, kind, │
│                                      parent, cwd) — cheap, no parse  │
│  LogDecoder (one per file)         → line → Vec<AgentEvent>          │
│                                      lenient: bad lines are counted  │
│                                      in ParseDiagnostics, skipped    │
└───────────────┬──────────────────────────────────────┬───────────────┘
      live path │                                      │ history path
                ▼                                      ▼
┌───────────────────────────────────┐  ┌───────────────────────────────┐
│ agtrace-runtime                   │  │ agtrace-index (SQLite, v7)    │
│  WorkspaceWatcher (one thread)    │  │  headers only: sessions +     │
│   · 250 ms: stat tracked files,   │  │  log_files pointers, agent    │
│     FileCursor tails new bytes    │  │  kind / parent / root columns │
│     (offset-based, partial lines  │  └───────────────┬───────────────┘
│     kept, truncation ⇒ reset)     │                  │ query
│   · 1 s: bounded discovery tick   │                  ▼
│     scoped by WatchScope          │  ┌───────────────────────────────┐
│   · side state: Claude registry,  │  │ agtrace-engine                │
│     team configs, subagent meta   │  │  decode files on demand →     │
└───────────────┬───────────────────┘  │  assemble sessions: turns,    │
                │ WorkspaceEvent       │  steps, per-agent streams     │
                ▼                      └───────────────┬───────────────┘
┌───────────────────────────────────┐                  │
│ agtrace-engine::workspace         │                  │
│  WorkspaceView fold (pure, O(1)   │                  │
│  per event): agent graph + handle │                  │
│  resolution, status derivation,   │                  │
│  feed (dedup), per-agent timeline,│                  │
│  context window resolver          │                  │
└───────────────┬───────────────────┘                  │
                ▼                                      ▼
┌──────────────────────────────────────────────────────────────────────┐
│ agtrace-sdk   Client::watch_workspace → LiveWorkspace (view +        │
│               generation counter)   ·   Client::sessions() / MCP     │
└───────────────┬──────────────────────────────────────────────────────┘
                ▼
┌──────────────────────────────────────────────────────────────────────┐
│ agtrace-cli   watch TUI / console · session show · lab · mcp serve   │
└──────────────────────────────────────────────────────────────────────┘
```

### Live path (`watch`)

1. **Discovery.** Once per second the `WorkspaceWatcher` lists only a bounded set of
   directories:
   - the project's Claude directory: top-level transcripts, plus `<sid>/subagents/` for
     tracked sessions;
   - today's and yesterday's Codex date directories;
   - `~/.claude/sessions/*.json` and the team configs of tracked teams.

   Each new file's header is read once. `WatchScope` then decides whether to track it:
   - `Project { root, since }`: roots whose cwd is under the project and that were active
     within `since` or have a live Claude process, plus all their descendants.
   - `Root(AgentId)`: one tree.
2. **Tailing.** Every 250 ms each tracked file is `stat`ed. A file that grew is read from its
   last offset by its `FileCursor`. Only complete lines are decoded; a partial last line waits
   for the next poll. Truncation or a change of file identity resets the cursor. The initial
   attach is the same cursor reading from offset 0, so batch and live decoding are identical.
3. **Fold.** The SDK folds `WorkspaceEvent`s into a `WorkspaceView`:
   - the agent graph (parents, children, links from `AgentSpawn` / team config / Codex
     `session_meta`);
   - each agent's status, current tool, recent timeline, and context evidence;
   - a workspace-wide message feed.

   It bumps a generation counter whenever the view changes.
4. **Render.** The CLI reads the view on each generation change (at most 10 fps) and draws the
   tree, focus pane and feed. See [watch](commands/watch.md).

### History path (`session`, `lab`, MCP)

1. `agtrace init` / `index update` walks the provider log roots using **headers only** and
   stores pointers in SQLite. Files whose size and mtime are unchanged are skipped.
2. A query resolves a session to its log files (the main file first, then Claude
   subagent / fork files). The engine decodes them on demand and assembles turns and steps,
   one stream per agent. It uses the typed `TurnEnd` and `Compaction` events.
3. The agent tree of a session comes from the index: `log_files` for Claude subagents and
   forks, and `sessions.parent_session_id` for teammates and Codex children.

### Context window resolution

The context window is resolved per agent by one function, `agtrace-engine::context::resolve`.
It checks, in order:

1. the user override (`[context_window]` in `config.toml`)
2. an explicit number in the log
3. an extended-context marker such as `[1m]`
4. the Codex models cache
5. the built-in model table
6. the observed peak usage

The providers supply the model catalog. The CLI never computes limits itself. See
[Supported Providers](providers.md#context-window).

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
use agtrace_sdk::{Client, watch::WatchScope};

let client = Client::connect_default().await?;
let mut live = client.watch_workspace(WatchScope::project(std::env::current_dir()?))?;
while live.changed().await.is_some() {
    let view = live.view();
    println!("{} agents, {} feed entries", view.agents.len(), view.feed.len());
}
```

### agtrace-types (Domain Model)

`AgentEvent` (with its `AgentId` and file origin), `AgentRef` / `AgentKind`, the event
payloads, and the provider decoder contract. Other crates depend on it; it depends on no other
agtrace crate.

- **Payloads:**
  - conversation: `User`, `Message`, `Reasoning`, `ToolCall`, `ToolResult`, `TokenUsage`
  - multi-agent: `AgentSpawn`, `AgentLifecycle`, `AgentMessage`
  - turn and context: `TurnEnd`, `Compaction`, `ModelChange`, `ContextWindowHint`
  - other: `AgentAttribute`, `ToolSubAction`, `Notification`, `SlashCommand`,
    `QueueOperation`
- **Contract:** `Provider`, `LogDecoder`, `FileHeader`, `ParseDiagnostics`.

### agtrace-providers (Adapter Layer)

Normalizes provider log formats into `AgentEvent`s.

**Responsibilities:**
- Discover agent files and read their headers, including sidecars: Claude subagent
  `.meta.json`, team configs and the session registry; the Codex session index and models cache
- Decode files line by line with a stateful per-file `LogDecoder`
- Map provider-specific tool names to standard types
- Supply the model catalog for the context window resolver

### agtrace-index (Storage Layer)

Maintains a lightweight SQLite database for fast session lookup.

**Responsibilities:**
- Store session and log-file pointers with agent metadata (see below)
- Enable fast queries like "show me all sessions for this project"
- Never duplicate log content

#### Index schema (v7)

The index is rebuilt automatically when the schema version changes; there is no migration.
Rows are built from **file headers only**.

- `sessions`: one row per session owner (Claude main or teammate, any Codex thread).
  - `agent_kind` (`main` | `teammate` | `codex_thread` | `fork`)
  - `agent_name`, `agent_path`, `team_name`
  - `root_session_id` and `parent_session_id` (no foreign key, so children may be indexed
    before their parents)
  - `spawn_call_id`
- `log_files`: every agent file of a session.
  - `role` (`main` | `subagent` | `fork`)
  - `agent_id`, `agent_name`, `spawn_call_id`, plus file size and mtime

  Claude subagents and forks are stored only here, under their parent session.
- A teammate's parent is the team config's `leadSessionId` when the config exists at index
  time. Otherwise it is left empty, and the live graph resolves it.

### agtrace-runtime (Orchestration)

- The `WorkspaceWatcher` and `FileCursor` tailer (live path).
- Index and query operations (history path).
- Configuration, including the `[context_window]` overrides.

### agtrace-engine (Domain Logic)

- **`workspace`:** the pure live fold. It covers the agent graph, handle resolution, status
  derivation, the feed, and the timelines.
- **`context`:** the context window resolver.
- **Session assembly:** turns and steps, per agent. Turns are closed by `TurnEnd` and marked
  by `Compaction`.
- **Diagnostic lenses:** Failures, Loops, Bottlenecks.

### agtrace-cli (Reference Application)

The official CLI application built on top of `agtrace-sdk`. Demonstrates best practices.

**Responsibilities:**
- The live multi-agent `watch` TUI and `--mode console`
- `session` commands for historical inspection, including agent trees
- `lab` commands for advanced queries; `mcp serve`
- `demo`: replays a synthetic workspace through the real watcher and TUI
- JSON export for programmatic use

## Storage Layout

```
System data directory/agtrace/  # e.g., ~/Library/Application Support/agtrace on macOS (or AGTRACE_PATH)
├── agtrace.db               # SQLite pointer index (schema v7)
└── config.toml              # Providers and [context_window] overrides

~/.claude/                   # Claude Code (read only; not modified by agtrace)
├── projects/<encoded-cwd>/<session-id>.jsonl
├── projects/<encoded-cwd>/<session-id>/subagents/agent-<id>.jsonl (+ .meta.json)
├── teams/<team>/config.json
└── sessions/<pid>.json

~/.codex/                    # Codex (read only; not modified by agtrace)
├── sessions/YYYY/MM/DD/rollout-<timestamp>-<thread-id>.jsonl
├── session_index.jsonl
└── models_cache.json
```

## Design Trade-offs

| Decision | Trade-off | Rationale |
|----------|-----------|-----------|
| Pointer-based index | Query latency vs storage | Log files can be gigabytes; duplication is prohibitive |
| Schema-on-read | Parse cost vs resilience | Provider schemas change frequently; reindexing is cheaper than data loss |
| CWD-based scoping | Simplicity vs flexibility | Most workflows are single-project; multi-project users can `cd` |
| SQLite for metadata | Deployment simplicity vs scale | Targets individual developers, not production observability |

## Future Directions

- **Cross-session comparison** to identify patterns across multiple runs
- **Export to observability platforms** for teams that want centralized dashboards
