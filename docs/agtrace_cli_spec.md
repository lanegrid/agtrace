# agtrace CLI Specification (v2.3 - Init Command & Smart Onboarding)

## 0. Overview

`agtrace` is a CLI tool for managing agent behavior logs from Claude Code / Codex / Gemini CLI through a lightweight indexing approach.

Instead of copying and converting logs, agtrace:
1. Scans provider log directories and registers metadata (pointers) in SQLite
2. Provides unified views by reading and normalizing raw logs on-demand (Schema-on-Read)
3. Offers CLI commands for listing, viewing, and managing sessions

Key principle: **"Pointer-based architecture"** - physical files remain untouched, only metadata is stored.

---

## 0.1 Core Concepts

### Provider

Claude Code / Codex / Gemini CLI - tools that generate agent behavior logs.

Default log roots:
- Claude: `$HOME/.claude/projects`
- Codex:  `$HOME/.codex/sessions`
- Gemini: `$HOME/.gemini/tmp`

Configuration is stored in `<data-dir>/config.toml` (default: `~/.agtrace/config.toml`):

```toml
[providers.claude]
enabled = true
log_root = "/Users/<user>/.claude/projects"

[providers.codex]
enabled = true
log_root = "/Users/<user>/.codex/sessions"

[providers.gemini]
enabled = true
log_root = "/Users/<user>/.gemini/tmp"
```

### Project

A source code repository unit that agtrace targets.

`project_root` is determined by:
1. `--project-root <PATH>` if specified
2. `AGTRACE_PROJECT_ROOT` environment variable
3. Current directory (`cwd`) as fallback

`project_hash` is `sha256(project_root).hex`

**Important: Project Isolation**
- Projects are identified by **exact matching** of `project_root` (for path-based providers like Claude/Codex) or `project_hash` (for hash-based providers like Gemini)
- **Subdirectories are treated as completely separate projects**
- **Parent directories are treated as completely separate projects**
- Rationale: Consistency across providers. Hash-based providers generate different values for each directory level, making hierarchical relationships impractical to maintain
- Example: `/project` and `/project/subdir` are separate projects with different hashes and should not be mixed

### Session

A logical unit of work (conversation or execution). The primary unit for UI listing.

---

## 1. Command Overview

### 1.1 Command Hierarchy

Commands are organized into namespaces for better discoverability:

* **`agtrace init`** - First-time setup (recommended starting point)
  - Auto-detect providers
  - Set up database
  - Scan for sessions
  - Show quick-start commands

* **`agtrace index`** - Database indexing operations
  - `update` - Scan and register new sessions
  - `rebuild` - Force full re-scan
  - `vacuum` - Optimize database

* **`agtrace session`** - Session management
  - `list` - List sessions
  - `show` - Display session timeline

* **`agtrace provider`** - Provider configuration
  - `list` - Show current configuration
  - `detect` - Auto-detect providers
  - `set` - Update provider settings
  - `schema` - Display provider schema

* **`agtrace doctor`** - Diagnostics and troubleshooting
  - `run` - Diagnose schema issues
  - `inspect` - View raw log files
  - `check` - Validate specific files

* **`agtrace project`** - Project information
  - `list` - Show registered projects

* **`agtrace lab`** - Experimental analysis features
  - `analyze` - Detect anti-patterns
  - `export` - Export sessions for analysis

**Legacy Commands (Deprecated):**

The following commands are still available for backwards compatibility but are deprecated. They will show a warning when used:
- `scan` → Use `index update` or `index rebuild` instead
- `list` → Use `session list` instead
- `view` / `show` → Use `session show` instead
- `providers` → Use `provider ...` instead
- `schema` → Use `provider schema` instead
- `diagnose` → Use `doctor run` instead
- `inspect` → Use `doctor inspect` instead
- `validate` → Use `doctor check` instead
- `analyze` → Use `lab analyze` instead
- `export` → Use `lab export` instead

**Removed (No longer available):**
- `find`: Legacy search command removed (used deprecated file-based storage)
- `show`: Legacy display command removed (different from `session show`, used deprecated storage)
- `stats`: Legacy statistics command removed (used deprecated file-based storage)
- `status`: Merged into `project` command

**Note:** These legacy commands relied on the deprecated v1 file-based storage system. Use the new hierarchical commands (`session show`, `lab analyze`, etc.) which use the SQLite-based pointer index.

### 1.2 Global Options

* `--data-dir <PATH>`
  - Description: Root directory for agtrace data (SQLite DB and config file)
  - Default: `$HOME/.agtrace`
  - Contents:
    - `<data-dir>/agtrace.db` - SQLite database
    - `<data-dir>/config.toml` - Configuration file

* `--format <plain|json>`
  - Description: CLI output format
  - Applied to: `list`, `view`, etc.
  - Default: `plain`

* `--log-level <error|warn|info|debug|trace>`
  - Description: CLI logging level
  - Default: `info`

* `--project-root <PATH>`
  - Description: Explicitly specify project root
  - Behavior: Sets `project_root` and calculates `project_hash = sha256(project_root)`
  - Impact: Default scope for `scan`, `list`, `view`

* `--all-projects`
  - Description: Disables project scope filtering
  - Behavior: Operates across all `project_hash` values
  - Note: `--project-hash` takes precedence if both are specified

* `--version`
  - Description: Display agtrace version

* `--help`
  - Description: Display help

### 1.3 Getting Started

#### Running `agtrace` without arguments

When you run `agtrace` without any subcommand, it displays context-aware guidance:

**First-time use (no config/database):**
```sh
$ agtrace

agtrace - Agent behavior log analyzer

Get started:
  agtrace init

The init command will:
  1. Detect and configure providers (Claude, Codex, Gemini)
  2. Set up the database
  3. Scan for sessions
  4. Show your recent sessions

For more commands:
  agtrace --help
```

**With sessions:**
```sh
$ agtrace

agtrace - Agent behavior log analyzer

Quick commands:
  agtrace session list              # View recent sessions
  agtrace session show <ID>         # View a session
  agtrace index update              # Scan for new sessions
  agtrace doctor run                # Diagnose issues

For more commands:
  agtrace --help
```

**No sessions found:**
```sh
$ agtrace

agtrace - Agent behavior log analyzer

No sessions found yet.

Next steps:
  agtrace index update              # Scan for sessions
  agtrace index update --all-projects  # Scan all projects
  agtrace provider list             # Check provider configuration

For more commands:
  agtrace --help
```

---

## 2. `agtrace init`

### 2.1 Overview

First-time setup command that streamlines the initial configuration and provides immediate value.

**Purpose:** Get from zero to viewing sessions in one command.

**What it does:**
1. Detects providers automatically (Claude, Codex, Gemini)
2. Creates `~/.agtrace/config.toml` if it doesn't exist
3. Sets up `~/.agtrace/agtrace.db` database
4. Scans for sessions (smart skip on subsequent runs)
5. Displays 10 most recent sessions
6. Shows next-step commands with actual session IDs

### 2.2 Signature

```sh
agtrace init \
  [--refresh] \
  [--all-projects]
```

### 2.3 Options

* `--refresh` (optional)
  - Description: Force re-scan even if recently scanned
  - Default: Skip scan if last scan was within 5 minutes
  - Use case: When you know new sessions were created

* `--all-projects` (optional, from global flags)
  - Description: Scan all projects instead of current project only
  - Default: Only scan current project (determined by `cwd`)
  - Use case: Initial setup when you want to see all your sessions

### 2.4 Behavior

**Smart Scan Skip:**
- On first run: Always scans
- On subsequent runs: Skips scan if `last_scanned_at` < 5 minutes ago
- With `--refresh`: Always scans regardless of `last_scanned_at`

**Project Scope:**
- Default: Only registers sessions for current project (`project_hash` from `cwd`)
- With `--all-projects`: Registers sessions for all detected projects
- Rationale: Focused experience by default, explicit flag for broad discovery

**No sessions found:**
- Without `--all-projects`: Suggests `agtrace init --all-projects`
- With `--all-projects`: Suggests checking provider config and running diagnostics

### 2.5 Output Example

**First run (current project only, no sessions):**
```sh
$ agtrace init

Initializing agtrace...

Step 1/4: Detecting providers...
  Detected 3 provider(s):
    claude -> /Users/user/.claude/projects
    codex -> /Users/user/.codex/sessions
    gemini -> /Users/user/.gemini/tmp
  Configuration saved to /Users/user/.agtrace/config.toml

Step 2/4: Setting up database...
  Database ready at /Users/user/.agtrace/agtrace.db

Step 3/4: Scanning for sessions...
Scan complete: 0 sessions registered

Step 4/4: Recent sessions...

No sessions found for the current project.

Tips:
  - Scan all projects: agtrace init --all-projects
  - Or: agtrace index update --all-projects
```

**First run with --all-projects:**
```sh
$ agtrace init --all-projects

Initializing agtrace...

Step 1/4: Detecting providers...
  Detected 3 provider(s):
    claude -> /Users/user/.claude/projects
    codex -> /Users/user/.codex/sessions
    gemini -> /Users/user/.gemini/tmp
  Configuration saved to /Users/user/.agtrace/config.toml

Step 2/4: Setting up database...
  Database ready at /Users/user/.agtrace/agtrace.db

Step 3/4: Scanning for sessions...
Scan complete: 276 sessions registered

Step 4/4: Recent sessions...

┌──────────────┬──────────┬──────────┬──────────────┬────────────────────┐
│ TIME         ┆ PROVIDER ┆ ID       ┆ PROJECT      ┆ SNIPPET            │
╞══════════════╪══════════╪══════════╪══════════════╪════════════════════╡
│ 5 min ago    ┆ claude   ┆ 25a42b97 ┆ 427e6b3fa... ┆ implement feature  │
│ 2 hours ago  ┆ codex    ┆ 019b04ae ┆ 88061af6f... ┆ refactor handlers  │
│ yesterday    ┆ gemini   ┆ f0a689a6 ┆ 2e4c1f...    ┆ add tests          │
└──────────────┴──────────┴──────────┴──────────────┴────────────────────┘

Next steps:
  View session in compact style (see bottlenecks and tool chains):
    agtrace session show 25a42b97 --style compact

  View conversation only (for LLM consumption):
    agtrace session show 25a42b97 --only user,assistant --full
```

**Second run (smart skip):**
```sh
$ agtrace init --all-projects

Initializing agtrace...

Step 1/4: Loading configuration...
  Configuration loaded from /Users/user/.agtrace/config.toml

Step 2/4: Setting up database...
  Database ready at /Users/user/.agtrace/agtrace.db

Step 3/4: Scanning for sessions...
  Recently scanned (2m ago). Skipping.
  Use `agtrace init --refresh` to force re-scan.

Step 4/4: Recent sessions...

┌──────────────┬──────────┬──────────┬──────────────┬────────────────────┐
│ TIME         ┆ PROVIDER ┆ ID       ┆ PROJECT      ┆ SNIPPET            │
...
```

### 2.6 Use Cases

**First-time installation:**
```sh
# Install agtrace, then:
agtrace init --all-projects
```

**Daily use (check for new sessions):**
```sh
# Runs quickly (skips scan if recent):
agtrace init
```

**Force refresh after long coding session:**
```sh
agtrace init --refresh
```

**Working in a specific project:**
```sh
cd /path/to/myproject
agtrace init  # Only shows sessions for this project
```

---

## 3. `agtrace index`

### 3.1 Overview

Manages the session index database through scanning and optimization operations.

**Key behavior:** Does not normalize or copy logs. Only stores metadata:
- File path (absolute)
- Session ID
- Project hash
- Timestamps
- File size / modification time

**Fail-safe:** If parsing errors occur, still registers the pointer (can retry normalization later in `session show`).

### 3.2 Subcommands

#### `agtrace index update`

Scans provider-specific log directories and registers new/modified sessions.

```sh
agtrace index update \
  [--provider <claude|codex|gemini|all>] \
  [--verbose]
```

**Options:**
* `--provider <claude|codex|gemini|all>` (optional)
  - Description: Target provider(s) to scan
  - Default: `all`
  - `all`: Scans all enabled providers in config.toml

* `--verbose`
  - Description: Display scan details

#### `agtrace index rebuild`

Force full re-scan of all sessions, ignoring modification times.

```sh
agtrace index rebuild \
  [--provider <claude|codex|gemini|all>] \
  [--verbose]
```

**Options:** Same as `update`

#### `agtrace index vacuum`

Optimize the SQLite database by reclaiming unused space.

```sh
agtrace index vacuum
```

### 3.3 Behavior

1. Reads `<data-dir>/config.toml` to determine provider `log_root`
2. Determines current `project_root` (via `--project-root`, `AGTRACE_PROJECT_ROOT`, or `cwd`)
3. For each provider:
   - Scans `log_root` for session files
   - Reads header information (session ID, timestamps, project info)
   - Matches sessions to current project (by `project_root` or `project_hash`)
   - Registers matching sessions in SQLite
4. Updates `last_scanned_at` timestamp for the project

**Scope:**
- Without `--all-projects`: Only registers sessions matching current `project_root`/`project_hash` (**exact match**)
- With `--all-projects`: Registers all detected sessions (useful for initial setup)

**Project Matching (Important):**
- Sessions are matched using **exact path matching** (for `cwd`-based providers like Claude) or **exact hash matching** (for `project_hash`-based providers like Gemini)
- **Subdirectories are treated as completely separate projects**
- **Parent directories are treated as completely separate projects**
- Rationale: Since `project_hash`-based providers (Gemini) generate different hash values for subdirectories, maintaining path-based relationships would be inconsistent and difficult
- Example:
  - If `project_root` is `/Users/user/myproject`, sessions from `/Users/user/myproject/subdir` are **NOT** registered
  - If you want to see sessions from subdirectories, use `--all-projects` and filter manually, or run scan with that subdirectory as the project root

### 3.4 Output Example

```text
Scanning provider: claude
  Found 3 sessions for project 427e6b3f...
  Registered: 038c47b8-a1b2-4c3d-8e9f-0123456789ab
  Registered: 1600ec8f-b2c3-4d5e-9f01-23456789abcd
  Registered: eb5ce482-c14c-4de5-b2c1-1f6ad5839f0f

Scanning provider: codex
  Found 5 sessions for project 427e6b3f...
  Registered: 019ac8c0-3e15-7082-947c-084528a26a26
  ...

Scan complete: 8 sessions registered
```

---

## 4. `agtrace session`

### 4.1 `agtrace session list`

Lists sessions from the SQLite index with enhanced UI.

**Features:**
- Color-coded providers (Claude=blue, Codex=green, Gemini=red)
- Relative time display ("2 hours ago", "yesterday")
- UTF-8 table formatting with proper borders
- Smart snippet truncation (removes noise, 70 char limit)

Default: Shows sessions for current project (determined by Project Discovery).

### 4.2 Signature

```sh
agtrace session list \
  [--project-hash <hash>] \
  [--source <claude|codex|gemini>] \
  [--limit <n>] \
  [--since <timestamp>] \
  [--until <timestamp>]
```

### 4.3 Options

* `--project-hash <hash>`
  - Description: Filter by specific project hash
  - Note: Use global `--project-root` or `--all-projects` for flexible filtering

* `--source <claude|codex|gemini>`
  - Description: Filter by specific provider

* `--limit <n>`
  - Description: Show most recent N sessions
  - Default: `50`

* `--since <timestamp>`
  - Description: Filter sessions after this timestamp

* `--until <timestamp>`
  - Description: Filter sessions before this timestamp

### 4.4 Output Example

```text
┌──────────────┬──────────┬──────────┬──────────────┬──────────────────────────────────────────┐
│ TIME         ┆ PROVIDER ┆ ID       ┆ PROJECT      ┆ SNIPPET                                  │
╞══════════════╪══════════╪══════════╪══════════════╪══════════════════════════════════════════╡
│ 2 hours ago  ┆ claude   ┆ 98176240 ┆ 427e6b3fa... ┆ read docs and evaluate sessions          │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ yesterday    ┆ codex    ┆ 019b04ae ┆ 427e6b3fa... ┆ add myapp directory                      │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ 3 days ago   ┆ gemini   ┆ f0a689a6 ┆ 427e6b3fa... ┆ [empty]                                  │
└──────────────┴──────────┴──────────┴──────────────┴──────────────────────────────────────────┘
```

**Note:** Colors are automatically disabled when output is piped.

---

### 4.5 `agtrace session show`

Displays session events by reading raw log files and normalizing them on-demand (Schema-on-Read).

**Features:**
- Color-coded event types (UserMessage=green, AssistantMessage=blue, ToolCall=yellow, etc.)
- Relative time from session start (`[+3s]`, `[+1m 23s]`)
- Smart text truncation (default 100 chars, disable with `--full`)
- Event filtering with `--hide` and `--only`
- Automatic color disable when piped (for LLM consumption)
- Session summary with token counts and duration

**Key behavior:**
1. Queries SQLite for file paths associated with `session_id`
2. Opens raw log files
3. Dynamically normalizes to `AgentEventV1`
4. Filters events based on `--hide`/`--only` options
5. Sorts/merges by timestamp
6. Displays in timeline format with optional colors

#### Signature

```sh
agtrace session show <SESSION_ID_OR_PREFIX> \
  [--raw] \
  [--json] \
  [--timeline] \
  [--hide <event_types>] \
  [--only <event_types>] \
  [--full] \
  [--style <timeline|compact>]
```

#### Options

* `<SESSION_ID_OR_PREFIX>` (required)
  - Session ID from `agtrace session list` (supports prefix matching)

* `--raw`
  - Description: Display raw JSON/text without normalization (like `cat`)

* `--json`
  - Description: Output normalized events as JSON array

* `--timeline`
  - Description: (Default) Human-readable timeline format

* `--hide <event_types>`
  - Description: Comma-separated list of event types to hide
  - Examples: `reasoning`, `tool`, `user,assistant`
  - Supported: `user`, `assistant`, `reasoning`, `tool` (matches both ToolCall and ToolResult)

* `--only <event_types>`
  - Description: Comma-separated list of event types to show (whitelist)
  - Examples: `user,assistant` (conversation only), `tool` (tool calls only)
  - Takes precedence over `--hide`

* `--full`
  - Description: Display full event text without truncation (useful for LLM consumption)
  - Default: Text is truncated at 100 characters

* `--style <timeline|compact>` (New in v2.1)
  - Description: Visualization style.
  - `timeline`: (Default) Standard vertical timeline with full event details.
  - `compact`: High-density one-line mode. Collapses repeated tool calls, highlights errors/latency, and visualizes flow.

#### Behavior

1. Lookup `session_id` in SQLite → retrieve all associated `log_files.path`
2. Open each file
3. Attempt normalization to `AgentEventV1`
4. If parsing fails, emit `{ type: "parse_error", raw: "..." }` and continue
5. Filter events based on `--hide` or `--only` options
6. Sort/merge events by timestamp
7. Detect if output is piped → disable colors if true
8. Display with appropriate formatting

**Auto color detection:**
- Terminal output → Colors enabled
- Piped output → Colors disabled (for LLM consumption)

#### Output Examples

#### Default timeline view (terminal)

```text
[+0s    ] UserMessage          (role=User)
  just say hi
  hi

[+3s    ] AssistantMessage     (role=Assistant)
  Hi! How can I help you today?
  tokens: in:1, out:12

---
Session Summary:
  Events: 2
    User messages: 1
    Assistant messages: 1
    Tool calls: 0
    Reasoning blocks: 0
  Tokens: 13
    Input: 1
    Output: 12
  Duration: 0m 3s
```

**Note:** In actual terminal output, event types are color-coded.

#### Conversation only (--only user,assistant)

```sh
agtrace session show <session_id> --only user,assistant
```

#### Full text for LLM (--full + pipe)

```sh
agtrace session show <session_id> --full --only user,assistant | pbcopy
```

This outputs complete event text without truncation and without ANSI color codes, perfect for pasting into LLM context.

#### Hide reasoning

```sh
agtrace session show <session_id> --hide reasoning
```

#### Compact view (`--style compact`)

Designed for quick scanning of long sessions to identify bottlenecks and loops using a **buffering state machine** that merges `Thinking → Tool` cycles into single lines.

**Format:** `[TIMESTAMP] [DURATION] [FLOW/CONTENT]`

  * **Timestamp:** Relative to session start, shows when the **action cycle started** (first Reasoning or ToolCall).
  * **Duration:**
      * Tool Chains: Total wall-clock time from cycle start to last ToolResult (e.g., `19s`, `201s`).
      * Messages: `-` placeholder.
  * **Flow/Content:**
      * **User/Assistant:** `Role: "Content start..."` (Truncated to 100 chars, newlines flattened to spaces, consecutive spaces normalized).
      * **Tool Chain:** `ToolA → ToolB(xN) → ToolC` (Sequential tools collapsed, `Thinking` absorbed).
      * **Status:** Color-coded duration (Yellow >10s, Red >30s).

**Key Behavior:**

  * **Thinking Absorption:** `Reasoning` events are merged into the following tool chain, not displayed separately. The timestamp shows when thinking started.
  * **Chain Buffering:** `Thinking → Tool1 → Tool2 → Tool3` becomes a single line, even if interleaved with `ToolResult` events.
  * **Flush Points:** Buffer is flushed (output printed) when `UserMessage` or `AssistantMessage` arrives.
  * **Input Summary:** Tool calls show **input** (command, pattern, file), not **output** (result). Examples:
    - `Bash(cargo build)` - shows the command
    - `Grep("error")` - shows the search pattern
    - `Edit(main.rs)` - shows the target file

```text
$ agtrace session show 6dae8df5 --style compact

[+00:00]    -    User: "新しいワークフローでスキーマ検証をしてみたい。docs を読んでもらって。 --- これで、UNIXコマンドを使わずに完全にagtrace内で問題解決できます： # 1. 問題発見 agtrace d..."
[+00:06]    -    Asst: "docsディレクトリを読んで、新しいスキーマ検証ワークフローを理解します。..."
[+00:05]  7s     Glob("docs/**/*.md") → Read(troubleshooting_schema_issues.md, agtrace_cli_spec.md)
[+00:36]    -    Asst: "docsを読みました。新しいスキーマ検証ワークフローを理解しました。 ## ワークフローの概要 `agtrace`内で完全に問題解決できる統合された診断フローです： ### 1. **問題発見** -..."
[+01:19]    -    User: "段階的に進めてください。遠回りしても良い"
[+01:27]    -    Asst: "わかりました。段階的に進めます。 まずは現在の状態を確認するところから始めましょう。 ## Step 1: ビルドして現在の状態を確認 最初にビルドして、`diagnose`コマンドがどんな問題を検出..."
[+01:25]  4s     Bash(cargo build --release)
[+01:35]    -    Asst: "ビルド成功しました。次に現在の問題を診断します："
[+01:36] 999ms   Bash(./target/release/agtrace di...)
[+02:14]  5s     Glob("src/**/*.rs") → Read(schema.rs)
[+07:08] 10s     Edit(schema.rs)
[+07:37] 149ms   Edit(io.rs)
[+07:49] 111ms   Edit(io.rs)
```

**Real-world compression examples** demonstrating fact-based folding with input summary:

```text
# Same file, repeated attempts
[+01:44] 23s   TodoWrite → Edit(schema.rs) → TodoWrite → Edit(schema.rs) → TodoWrite → Edit(schema.rs)

# Multiple files in sequence with commands
[+12:38] 44s   Glob("src/**/*.rs") → Grep("sample_size") → Read(args.rs) → Edit(args.rs) → Read(commands.rs) → Edit(commands.rs, diagnose.rs x2)

# Complex 201-second workflow with full context
[+06:38] 201s  Glob("src/cli/handlers/...") → Grep("fn can_handle") → Read(mod.rs x2) → TodoWrite(x2) → Edit(mod.rs) → Read(mod.rs) → Grep("fn is_empty_codex...") → TodoWrite → Edit(mod.rs) → Read(mod.rs) → TodoWrite → Edit(mod.rs) → TodoWrite → Edit(diagnose.rs x3) → TodoWrite → Bash(cargo build --release, ./target/release/agtrace di...) → TodoWrite

# Build and diagnostic commands
[+00:06]  8s   Bash(cargo build --release, ./target/release/agtrace di...)
[+02:15]  3s   TodoWrite → Bash(cargo build --release)
[+05:31]  5s   Grep("empty_file") → Read(diagnose.rs)
```

**Key Features (Fact-Based Folding + Input Summary):**

  * **Input Summary, Not Output:** Shows what was **asked** (command, pattern, file), not the **result**:
    - `Bash(cargo build)` - not the compiler output
    - `Grep("error")` - not the search results
    - `Edit(schema.rs)` - not the file contents
  * **Target Transparency:** File names are preserved (e.g., `Edit(schema.rs)`, `Read(mod.rs x2)`)
  * **Command Visibility:** `Bash` shows actual commands: `Bash(cargo build --release)`, `Bash(git status)`
  * **Pattern Visibility:** Search tools show patterns: `Grep("empty_file")`, `Glob("src/**/*.rs")`
  * **Contextual Compression:** `schema.rs x4` means "same file, 4 times in a row"
  * **Multi-Target Display:** `Edit(commands.rs, diagnose.rs x2)` shows multiple files in order
  * **No Interpretation:** Tool doesn't judge "沼" vs "deliberate work" - user decides based on context
  * **Extreme Density:** 201s workflow with 20+ operations fits in one line
  * **Execution Order Preserved:** `Edit(a) → Read(b) → Edit(a)` keeps sequence, doesn't merge to `Edit(a x2)`

---

## 5. `agtrace project`

### 5.1 `agtrace project list`

Displays project information and registered session counts.

#### Signature

```sh
agtrace project list
```

#### Output Example

```text
Registered projects:

HASH              ROOT PATH                           SESSIONS  LAST SCANNED
427e6b3f...       /Users/user/projects/agtrace        12        2025-12-10 19:55:16
2e4c1f...         /Users/user/projects/transcene      5         2025-12-09 18:30:00
```

---

## 6. `agtrace provider`

### 6.1 Overview

Manages provider configuration (view/detect/update).

### 6.2 Subcommands

#### `agtrace provider list`

List current provider configuration.

```sh
agtrace provider list
```

Reads `<data-dir>/config.toml` and displays `providers.*` sections.

#### `agtrace provider detect`

Auto-detect providers and write to config.

```sh
agtrace provider detect
```

Searches `$HOME/.claude`, `$HOME/.codex`, `$HOME/.gemini` and writes detected providers to config with `enabled = true`.

#### `agtrace provider set`

Update provider settings.

```sh
agtrace provider set <PROVIDER> --log-root <PATH> [--enable|--disable]
```

Updates `log_root` and `enabled` flag for specified provider.

#### `agtrace provider schema`

Display provider schema structure.

```sh
agtrace provider schema <PROVIDER> [--format <text|json|rust>]
```

See section 5.3 for details.

### 6.3 `agtrace provider schema`

Displays the expected schema structure for a provider. Useful reference when fixing schema compatibility issues.

#### Signature

```sh
agtrace provider schema <PROVIDER> \
  [--format <text|json|rust>]
```

#### Options

* `<PROVIDER>` (required)
  - Provider name: `claude`, `codex`, or `gemini`

* `--format <text|json|rust>` (optional)
  - `text`: Human-readable description (default)
  - `json`: JSON Schema format
  - `rust`: Rust struct definitions

#### Output Examples

**Text format (default):**
```text
$ agtrace provider schema codex

Provider: Codex
Schema version: v0.53-v0.63

Root structure (JSONL - one record per line):
  CodexRecord (enum):
    - SessionMeta
    - ResponseItem
    - EventMsg
    - TurnContext

SessionMeta:
  timestamp: String
  payload:
    id: String (session_id)
    cwd: String
    originator: String
    cli_version: String
    source: String | Object
    model_provider: String
    git: GitInfo (optional)

TurnContext:
  timestamp: String
  payload:
    cwd: String
    approval_policy: String
    sandbox_policy: SandboxPolicy (see below)
    model: String
    effort: String
    summary: String

SandboxPolicy (untagged enum):
  New format (v0.63+):
    { "type": "read-only" | "workspace-write" }
  Old format (v0.53):
    { "mode": "...", "network_access": bool, ... }
```

**Rust format:**
```text
$ agtrace provider schema codex --format rust

// src/providers/codex/schema.rs

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum CodexRecord {
    SessionMeta(SessionMetaRecord),
    ResponseItem(ResponseItemRecord),
    EventMsg(EventMsgRecord),
    TurnContext(TurnContextRecord),
    #[serde(other)]
    Unknown,
}

...
```

---

## 7. `agtrace doctor`

### 7.1 `agtrace doctor run`

Diagnoses schema compatibility issues by sampling log files from each provider. This command helps identify:
- Files that fail to parse due to schema mismatches
- Missing required fields
- Type mismatches
- Version-specific format changes

**Purpose:** Make debugging schema issues trivial - run once to understand all parsing problems across providers.

#### Signature

```sh
agtrace doctor run \
  [--provider <claude|codex|gemini|all>] \
  [--verbose]
```

#### Options

* `--provider <claude|codex|gemini|all>` (optional)
  - Description: Target provider(s) to diagnose
  - Default: `all`

* `--verbose`
  - Description: Show all problematic files (not just examples)

#### Behavior

1. Scans provider log directories (using config.toml)
2. Collects **all files** for each provider (no sampling)
3. Attempts to parse each file with current schema
4. Categorizes failures by error type:
   - `missing_field`: Required field not present
   - `type_mismatch`: Field type doesn't match schema
   - `parse_error`: JSON syntax error or unknown structure
   - `empty_file`: File has no meaningful content
5. Aggregates results and displays summary with examples

**Note:** This command processes all files comprehensively to ensure no issues are missed. For large log directories with hundreds of files, this may take a few seconds.

#### Output Format

**Default output (aggregated):**

```text
=== Diagnose Results ===

Provider: Claude
  Total files scanned: 329
  Successfully parsed: 312 (94.8%)
  Parse failures: 17 (5.2%)

  Failure breakdown:
  ✗ empty_file: 16 files
    Example: /Users/.../a50cd2c1-d8df-4ae7-ae5d-887009d66940.jsonl
    Reason: No events extracted from file

    ... and 15 more files

  ✗ parse_error: 1 files
    Example: /Users/.../374bc3d8-9eaf-4419-897c-dd84881047a9.jsonl
    Reason: Failed to parse JSON line: ...

Provider: Codex
  Total files scanned: 81
  Successfully parsed: 48 (59.3%)
  Parse failures: 33 (40.7%)

  Failure breakdown:
  ✗ missing_field (model_provider): 19 files
    Example: /Users/.../rollout-2025-10-28T16-24-01-019a29b3-d031-7b31-9f2d-8970fd673604.jsonl
    Reason: Missing required field: model_provider

    ... and 18 more files

  ✗ missing_field (effort): 14 files
    Example: /Users/.../rollout-2025-11-03T10-46-11-019a4764-ae62-7042-9514-01a47b61b8e5.jsonl
    Reason: Missing required field: effort

    ... and 13 more files

Provider: Gemini
  Total files scanned: 12
  Successfully parsed: 11 (91.7%)
  Parse failures: 1 (8.3%)

  Failure breakdown:
  ✗ empty_file: 1 files
    Example: /Users/.../a7e6a102cb8d98a9665a366914d81fc84cb6e3264be0970c66e14288b15761d7/logs.json
    Reason: No events extracted from file

---
Summary: 51 files need schema updates to parse correctly
Run with --verbose to see all problematic files
```

**Verbose output:**

Shows all files in each category (not just examples)

#### Use Cases

**Regular health check (all providers):**
```sh
agtrace doctor run
```

**Debug specific provider with full details:**
```sh
agtrace doctor run --provider codex --verbose
```

**Quick check for a single provider:**
```sh
agtrace doctor run --provider gemini
```

---

### 7.2 `agtrace doctor inspect`

Displays the raw content of a log file with line numbers for manual inspection. Useful for examining actual data structure when debugging schema issues.

#### Signature

```sh
agtrace doctor inspect <FILE_PATH> \
  [--lines <n>] \
  [--format <raw|json>]
```

#### Options

* `<FILE_PATH>` (required)
  - Path to the log file to inspect

* `--lines <n>` (optional)
  - Number of lines to display from the beginning
  - Default: `50`

* `--format <raw|json>` (optional)
  - `raw`: Display as-is with line numbers
  - `json`: Pretty-print JSON if valid
  - Default: `raw`

#### Output Example

```text
$ agtrace doctor inspect /Users/.../rollout-2025-12-04...jsonl --lines 5

File: /Users/.../rollout-2025-12-04...jsonl
Lines: 1-5 (total: 150 lines)
───────────────────────────────────────
     1  {"timestamp":"2025-12-04T13:23:36.135Z","type":"session_meta","payload":{"id":"019ae988...
     2  {"timestamp":"2025-12-04T13:23:36.136Z","type":"response_item","payload":{"type":"message...
     3  {"timestamp":"2025-12-04T13:23:36.148Z","type":"response_item","payload":{"type":"message...
     4  {"timestamp":"2025-12-04T13:23:36.148Z","type":"event_msg","payload":{"type":"user_messag...
     5  {"timestamp":"2025-12-04T13:23:36.153Z","type":"turn_context","payload":{"cwd":"/Users/za...
───────────────────────────────────────
```

---

### 7.3 `agtrace doctor check`

Validates a single log file against the provider's schema. Shows detailed parse errors and suggests fixes.

#### Signature

```sh
agtrace doctor check <FILE_PATH> \
  [--provider <claude|codex|gemini>]
```

#### Options

* `<FILE_PATH>` (required)
  - Path to the log file to validate

* `--provider <claude|codex|gemini>` (optional)
  - Explicitly specify provider (auto-detected from path if not provided)

#### Behavior

1. Auto-detect provider from file path if not specified
2. Attempt to parse file with provider's schema
3. Display detailed error information if parsing fails
4. Show expected schema structure vs. actual data

#### Output Examples

**Success case:**
```text
$ agtrace doctor check /Users/.../rollout-2025-12-04...jsonl

File: /Users/.../rollout-2025-12-04...jsonl
Provider: codex (auto-detected)
Status: ✓ Valid

Parsed successfully:
  - Session ID: 019ae988-502c-7533-a763-5c796e30804c
  - Events extracted: 45
  - Project: /Users/zawakin/go/src/github.com/lanegrid/pdna
```

**Failure case:**
```text
$ agtrace doctor check /Users/.../logs.json --provider gemini

File: /Users/.../logs.json
Provider: gemini
Status: ✗ Invalid

Parse error at line 2, column 2:
  invalid type: map, expected a string

Expected schema:
  GeminiSession {
    session_id: String,
    project_hash: String,
    start_time: String,
    last_updated: String,
    messages: [GeminiMessage]
  }

Actual structure (first record):
  {
    "sessionId": "f0a689a6...",  // ← Found in array, not root object
    "messageId": 0,
    "type": "user",
    ...
  }

Suggestion:
  File contains an array of messages, but expected a session object with metadata.
  This format may be from an older version of Gemini CLI.
```

---

## 8. Error Codes

* `0` … Success
* `1` … General error (parse failure / invalid input)
* `2` … Input path does not exist / not readable
* `3` … Storage write error
* `4` … Internal error (bug)

---

## 9. `agtrace lab`

### 9.1 `agtrace lab analyze`

Applies deterministic heuristics to `AgentEventV1` streams to detect behavioral anti-patterns and extract "Rule of Thumb" insights without using an LLM.

#### Signature

```sh
agtrace lab analyze <SESSION_ID_OR_PREFIX> \
  [--detect <patterns>] \
  [--format <plain|json>]
```

#### Options

  * `--detect <patterns>`
      * Description: Comma-separated list of detectors to run.
      * Default: `all`
      * Available detectors:
          * `loops`: Detects repeated tool sequences (e.g., Edit -> Fail -> Edit -> Fail).
          * `apology`: Detects excessive apologies ("I apologize", "My mistake").
          * `lazy_tool`: Detects tool calls that ignore previous errors without reasoning.
          * `zombie`: Detects long chains of tool calls (>20) without user interaction.
          * `lint_ping_pong`: Detects oscillation between coding and linting errors.

#### Output Example

```text
$ agtrace lab analyze f0a689

Analysis Report for Session: f0a689...
Score: 85/100 (1 Warning)

[WARN] Loop Detected (Count: 3)
  Span: +00:17 to +01:45
  Pattern: Edit(auth.ts) → Test(fail)
  Insight: Agent is struggling to fix the test. Consider reverting or creating a reproduction script.

[INFO] Tool Usage
  - Read: 12 times (Avg 150ms)
  - Edit: 4 times
  - Test: 4 times (Avg 15s) ← High Latency
```

---

### 9.2 `agtrace lab export`

Exports normalized session data for external use (training, analysis, or archiving).
Includes **"Distillation Strategies"** to produce higher-quality data than raw logs.

#### Signature

```sh
agtrace lab export <SESSION_ID_OR_PREFIX> \
  [--output <path>] \
  [--format <jsonl|text>] \
  [--strategy <raw|clean|reasoning>]
```

#### Options

  * `--format <jsonl|text>`

      * `jsonl`: Standard JSONL (one `AgentEventV1` per line).
      * `text`: Human-readable transcript.

  * `--strategy <raw|clean|reasoning>`

      * Description: Transformation strategy applied before export.
      * `raw`: (Default) Exports all events as-is.
      * `clean`: **"Gold Mining" mode.** Removes failed tool calls, error corrections, and apology turns. Keeps only the "happy path" (if reachable). Useful for SFT (Supervised Fine-Tuning) datasets.
      * `reasoning`: Extracts only `Thinking (Reasoning)` → `Tool Call` pairs. Useful for analyzing "Thought-Action" correlation.

#### Behavior (Strategy Details)

**Strategy: `clean` (The "Happy Path" Filter)**

1.  Identify the final state of the session.
2.  Backtrack and remove "dead ends" (e.g., tool calls that resulted in Error and were immediately followed by an Apology/Correction).
3.  Remove `tool_result` contents that are excessively long, replacing them with `<truncated_output_for_training>`.

#### Output Example

```bash
# Export a clean dataset for training
agtrace lab export f0a689 --strategy clean --output ./training_data/session_f0a689.jsonl

# Export reasoning traces for analysis
agtrace lab export f0a689 --strategy reasoning --output ./analysis/thoughts.jsonl
```

---

## 10. Future Extensions

  * `agtrace graph` - DAG visualization of session branches.
  * `agtrace diff` - Compare two sessions side-by-side (e.g., same prompt, different models).
  * `agtrace config` - Interactive configuration wizard.

---

This specification defines the **agtrace CLI v2.3 (Init Command & Smart Onboarding)** with streamlined first-time setup, smart scan skip, and context-aware guidance.

## Value Proposition of Compact View

### Fact-Based, Not Interpretive

The compact view delivers **compressed facts**, not judgments:

1.  **Context-Aware Transparency:**
    - `Edit(schema.rs x3)` = Same file, 3 consecutive edits
    - `Edit(a.rs, b.rs, c.rs)` = 3 different files in sequence
    - `Edit(a.rs) → Read(b.rs) → Edit(a.rs)` = Interleaved, NOT merged

2.  **User Decides Intent:**
    - `Edit(schema.rs x4)` could be:
      - ✓ Deliberate: "4 staged refinements of schema design"
      - ✗ 沼 (Stuck): "Failed 3 times, finally got it on 4th attempt"
    - Tool shows the fact; **you** know the code, so **you** interpret.

3.  **Cost & Bottleneck Awareness:**
    - `[+06:38] 201s` (red highlight) = Immediate visual cue for expensive operations
    - `Glob → Grep → Read(x2)` taking 48s → Consider search tool optimization

4.  **Rhythm Understanding:**
    - User ↔ Assistant dialogue is cleanly separated from tool execution chains
    - Long tool chains (201s, 20+ tools) indicate deep work cycles vs quick interactions
