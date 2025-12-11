# agtrace CLI Specification (v2.0 - Pointer Edition)

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

### 1.1 Command List

* **`agtrace scan`**
  Scans provider log directories and registers metadata (pointers) in the database.

* **`agtrace list`**
  Lists sessions from the index.

* **`agtrace view`** (formerly `show`)
  Displays session events by dynamically reading and normalizing raw logs (Schema-on-Read).

* **`agtrace project`**
  Shows project information.

* **`agtrace providers`**
  Manages provider configuration.

* **`agtrace diagnose`**
  Diagnoses schema compatibility issues by sampling log files.

* **`agtrace inspect`**
  Displays raw content of a log file with line numbers.

* **`agtrace validate`**
  Validates a single log file against provider schema.

* **`agtrace schema`**
  Displays expected schema structure for a provider.

**Removed in v2.0:**
- `find`: Removed for MVP (can be re-added later)
- `stats`: Removed for MVP (can be re-added later)
- `export`: Removed for MVP (can be re-added later)
- `status`: Merged into `project` command

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

---

## 2. `agtrace scan`

### 2.1 Overview

Scans provider-specific log directories, reads header information only, and registers pointers in the SQLite database.

**Key behavior:** Does not normalize or copy logs. Only stores metadata:
- File path (absolute)
- Session ID
- Project hash
- Timestamps
- File size / modification time

**Fail-safe:** If parsing errors occur, still registers the pointer (can retry normalization later in `view`).

### 2.2 Signature

```sh
agtrace scan \
  [--provider <claude|codex|gemini|all>] \
  [--force] \
  [--verbose]
```

### 2.3 Options

* `--provider <claude|codex|gemini|all>` (optional)
  - Description: Target provider(s) to scan
  - Default: `all`
  - `all`: Scans all enabled providers in config.toml

* `--force`
  - Description: Force full re-scan (ignores modification time)

* `--verbose`
  - Description: Display scan details

### 2.4 Behavior

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

### 2.5 Output Example

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

## 3. `agtrace list`

### 3.1 Overview

Lists sessions from the SQLite index with enhanced UI.

**Features:**
- Color-coded providers (Claude=blue, Codex=green, Gemini=red)
- Relative time display ("2 hours ago", "yesterday")
- UTF-8 table formatting with proper borders
- Smart snippet truncation (removes noise, 70 char limit)

Default: Shows sessions for current project (determined by Project Discovery).

### 3.2 Signature

```sh
agtrace list \
  [--project-hash <hash>] \
  [--source <claude|codex|gemini>] \
  [--limit <n>] \
  [--since <timestamp>] \
  [--until <timestamp>]
```

### 3.3 Options

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

### 3.4 Output Example

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

## 4. `agtrace view` (formerly `show`)

### 4.1 Overview

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

### 4.2 Signature

```sh
agtrace view <SESSION_ID_OR_PREFIX> \
  [--raw] \
  [--json] \
  [--timeline] \
  [--hide <event_types>] \
  [--only <event_types>] \
  [--full]
```

### 4.3 Options

* `<SESSION_ID_OR_PREFIX>` (required)
  - Session ID from `agtrace list` (supports prefix matching)

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

### 4.4 Behavior

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

### 4.5 Output Examples

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
agtrace view <session_id> --only user,assistant
```

#### Full text for LLM (--full + pipe)

```sh
agtrace view <session_id> --full --only user,assistant | pbcopy
```

This outputs complete event text without truncation and without ANSI color codes, perfect for pasting into LLM context.

#### Hide reasoning

```sh
agtrace view <session_id> --hide reasoning
```

---

## 5. `agtrace project`

### 5.1 Overview

Displays project information and registered session counts.

### 5.2 Signature

```sh
agtrace project list
```

### 5.3 Output Example

```text
Registered projects:

HASH              ROOT PATH                           SESSIONS  LAST SCANNED
427e6b3f...       /Users/user/projects/agtrace        12        2025-12-10 19:55:16
2e4c1f...         /Users/user/projects/transcene      5         2025-12-09 18:30:00
```

---

## 6. `agtrace providers`

### 6.1 Overview

Manages provider configuration (view/detect/update).

### 6.2 Signature

```sh
agtrace providers          # List current configuration
agtrace providers detect   # Auto-detect and write to config
agtrace providers set <PROVIDER> --log-root <PATH> [--enable|--disable]
```

### 6.3 Behavior

* `agtrace providers`:
  - Reads `<data-dir>/config.toml` and displays `providers.*` sections

* `agtrace providers detect`:
  - Searches `$HOME/.claude`, `$HOME/.codex`, `$HOME/.gemini`
  - Writes detected providers to config with `enabled = true`

* `agtrace providers set`:
  - Updates `log_root` and `enabled` flag for specified provider

---

## 7. `agtrace diagnose`

### 7.1 Overview

Diagnoses schema compatibility issues by sampling log files from each provider. This command helps identify:
- Files that fail to parse due to schema mismatches
- Missing required fields
- Type mismatches
- Version-specific format changes

**Purpose:** Make debugging schema issues trivial - run once to understand all parsing problems across providers.

### 7.2 Signature

```sh
agtrace diagnose \
  [--provider <claude|codex|gemini|all>] \
  [--sample-size <n>] \
  [--verbose]
```

### 7.3 Options

* `--provider <claude|codex|gemini|all>` (optional)
  - Description: Target provider(s) to diagnose
  - Default: `all`

* `--sample-size <n>` (optional)
  - Description: Number of files to sample per provider
  - Default: `10`

* `--verbose`
  - Description: Show all problematic files (not just samples)

### 7.4 Behavior

1. Scans provider log directories (using config.toml)
2. Samples N files per provider (random or recent)
3. Attempts to parse each file with current schema
4. Categorizes failures by error type:
   - `missing_field`: Required field not present
   - `type_mismatch`: Field type doesn't match schema
   - `parse_error`: JSON syntax error or unknown structure
   - `empty_file`: File has no meaningful content
5. Aggregates results and displays summary with examples

### 7.5 Output Format

#### Default output (aggregated):

```text
=== Diagnose Results ===

Provider: Codex
  Total files scanned: 150
  Successfully parsed: 120 (80.0%)
  Parse failures: 30 (20.0%)

  Failure breakdown:
  ✗ missing_field (source): 15 files
    Example: /Users/.../rollout-2025-11-11T00-49-22-019a6e75-2585-7540-9982-9dced67f1132.jsonl
    Reason: Field 'source' expected string, found object {"subagent":"review"}

  ✗ missing_field (session_id): 10 files
    Example: /Users/.../rollout-2025-10-28T16-24-01-019a29b3-d031-7b31-9f2d-8970fd673604.jsonl
    Reason: No SessionMeta record found in first 20 lines

  ✗ empty_file: 5 files
    Example: /Users/.../rollout-2025-09-15T02-36-25-58f46532-3eee-42e9-8060-d5152c6f66da.jsonl

Provider: Claude
  Total files scanned: 80
  Successfully parsed: 79 (98.8%)
  Parse failures: 1 (1.2%)

  Failure breakdown:
  ✗ missing_field (session_id): 1 file
    Example: /Users/.../81ef0c4f-d76a-415b-a6f6-e51fa6bc7d96.jsonl
    Reason: File contains only 'summary' and 'file-history-snapshot' records

Provider: Gemini
  Total files scanned: 4
  Successfully parsed: 3 (75.0%)
  Parse failures: 1 (25.0%)

  Failure breakdown:
  ✗ parse_error: 1 file
    Example: /Users/.../427e6b3f.../logs.json
    Reason: Array format instead of expected session object

---
Summary: 34 files need schema updates to parse correctly
Run with --verbose to see all problematic files
```

#### Verbose output:

Shows all files in each category (not just examples)

### 7.6 Use Cases

**Regular health check:**
```sh
agtrace diagnose
```

**Debug specific provider:**
```sh
agtrace diagnose --provider codex --verbose
```

**Quick sampling (faster):**
```sh
agtrace diagnose --sample-size 5
```

---

## 8. `agtrace inspect`

### 8.1 Overview

Displays the raw content of a log file with line numbers for manual inspection. Useful for examining actual data structure when debugging schema issues.

### 8.2 Signature

```sh
agtrace inspect <FILE_PATH> \
  [--lines <n>] \
  [--format <raw|json>]
```

### 8.3 Options

* `<FILE_PATH>` (required)
  - Path to the log file to inspect

* `--lines <n>` (optional)
  - Number of lines to display from the beginning
  - Default: `50`

* `--format <raw|json>` (optional)
  - `raw`: Display as-is with line numbers
  - `json`: Pretty-print JSON if valid
  - Default: `raw`

### 8.4 Output Example

```text
$ agtrace inspect /Users/.../rollout-2025-12-04...jsonl --lines 5

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

## 9. `agtrace validate`

### 9.1 Overview

Validates a single log file against the provider's schema. Shows detailed parse errors and suggests fixes.

### 9.2 Signature

```sh
agtrace validate <FILE_PATH> \
  [--provider <claude|codex|gemini>]
```

### 9.3 Options

* `<FILE_PATH>` (required)
  - Path to the log file to validate

* `--provider <claude|codex|gemini>` (optional)
  - Explicitly specify provider (auto-detected from path if not provided)

### 9.4 Behavior

1. Auto-detect provider from file path if not specified
2. Attempt to parse file with provider's schema
3. Display detailed error information if parsing fails
4. Show expected schema structure vs. actual data

### 9.5 Output Examples

#### Success case:
```text
$ agtrace validate /Users/.../rollout-2025-12-04...jsonl

File: /Users/.../rollout-2025-12-04...jsonl
Provider: codex (auto-detected)
Status: ✓ Valid

Parsed successfully:
  - Session ID: 019ae988-502c-7533-a763-5c796e30804c
  - Events extracted: 45
  - Project: /Users/zawakin/go/src/github.com/lanegrid/pdna
```

#### Failure case:
```text
$ agtrace validate /Users/.../logs.json --provider gemini

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

## 10. `agtrace schema`

### 10.1 Overview

Displays the expected schema structure for a provider. Useful reference when fixing schema compatibility issues.

### 10.2 Signature

```sh
agtrace schema <PROVIDER> \
  [--format <text|json|rust>]
```

### 10.3 Options

* `<PROVIDER>` (required)
  - Provider name: `claude`, `codex`, or `gemini`

* `--format <text|json|rust>` (optional)
  - `text`: Human-readable description (default)
  - `json`: JSON Schema format
  - `rust`: Rust struct definitions

### 10.4 Output Examples

#### Text format (default):
```text
$ agtrace schema codex

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

#### Rust format:
```text
$ agtrace schema codex --format rust

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

## 11. Error Codes

* `0` … Success
* `1` … General error (parse failure / invalid input)
* `2` … Input path does not exist / not readable
* `3` … Storage write error
* `4` … Internal error (bug)

---

## 12. Future Extensions (Not in v2.0 MVP)

* `agtrace find` - Full-text search across events
* `agtrace stats` - Token/tool usage statistics
* `agtrace export` - Export to JSONL/CSV
* `agtrace graph` - DAG visualization
* `agtrace diff` - Session comparison

---

This specification defines the **agtrace CLI v2.0 (Pointer Edition)** with a Schema-on-Read architecture.
