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

Lists sessions from the SQLite index.

Default: Shows sessions for current project (determined by Project Discovery).

### 3.2 Signature

```sh
agtrace list \
  [--project <root_path>] \
  [--hash <hash>] \
  [--recent <n>] \
  [--all]
```

### 3.3 Options

* `--project <root_path>`
  - Description: Filter by specific project path

* `--hash <hash>`
  - Description: Filter by specific project hash

* `--recent <n>`
  - Description: Show most recent N sessions
  - Default: `20`

* `--all`
  - Description: Show sessions from all projects

### 3.4 Output Example

```text
TIME                 PROVIDER  ID (short)  PROJECT              SNIPPET
2025-12-10 19:55:16  codex     019b04ae    agent-sample (hash)  add myapp dir...
2025-12-09 19:50:00  gemini    f0a689a6    agent-sample (hash)  add myapp dir...
2025-12-09 19:47:42  claude    7f2abd2d    agent-sample (hash)  add myapp dir...
```

---

## 4. `agtrace view` (formerly `show`)

### 4.1 Overview

Displays session events by reading raw log files and normalizing them on-demand (Schema-on-Read).

**Key behavior:**
1. Queries SQLite for file paths associated with `session_id`
2. Opens raw log files
3. Dynamically normalizes to `AgentEventV1`
4. Sorts/merges by timestamp
5. Displays in timeline format

### 4.2 Signature

```sh
agtrace view <SESSION_ID_OR_PREFIX> \
  [--raw] \
  [--json] \
  [--timeline]
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

### 4.4 Behavior

1. Lookup `session_id` in SQLite → retrieve all associated `log_files.path`
2. Open each file
3. Attempt normalization to `AgentEventV1`
4. If parsing fails, emit `{ type: "parse_error", raw: "..." }` and continue
5. Sort/merge events by timestamp
6. Display

### 4.5 Output Example (--timeline)

```text
[2025-11-03T01:49:22.517Z] user_message       U1   (role=user)
  summary this repo

[2025-11-03T01:49:23.073Z] reasoning          R1   (role=assistant)
  Plan: read README, scan src/, then propose a summary...

[2025-11-03T01:49:25.212Z] tool_call          T1   (shell)
  rg "agtrace" -n

[2025-11-03T01:49:26.836Z] tool_result        TR1  (shell, status=success)
  README.md:1: # agtrace
  ...
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

## 7. Error Codes

* `0` … Success
* `1` … General error (parse failure / invalid input)
* `2` … Input path does not exist / not readable
* `3` … Storage write error
* `4` … Internal error (bug)

---

## 8. Future Extensions (Not in v2.0 MVP)

* `agtrace find` - Full-text search across events
* `agtrace stats` - Token/tool usage statistics
* `agtrace export` - Export to JSONL/CSV
* `agtrace graph` - DAG visualization
* `agtrace diff` - Session comparison

---

This specification defines the **agtrace CLI v2.0 (Pointer Edition)** with a Schema-on-Read architecture.
