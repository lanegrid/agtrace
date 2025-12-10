# agtrace Architecture (v2.0 - Pointer Edition)

## Overview

agtrace v2.0 adopts a **Schema-on-Read** architecture, where raw log files remain untouched and normalization happens dynamically at view time.

**Design Philosophy:**
- **Lightweight Index:** Store only metadata (pointers) in SQLite, not full event data
- **Lazy Normalization:** Parse and normalize logs only when explicitly requested
- **Fail-safe:** Parsing errors during scan don't block indexing
- **Fast Iteration:** Re-scan is cheap (only reads headers), enabling incremental updates

---

## System Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                    Provider Log Directories                  │
│  ~/.claude/projects/  ~/.codex/sessions/  ~/.gemini/tmp/    │
└────────────────┬────────────────────────────────────────────┘
                 │
                 │ (1) Scan (Write-path)
                 │     Read header only
                 │     Extract: session_id, timestamps, project info
                 ▼
┌─────────────────────────────────────────────────────────────┐
│            SQLite Index (<data-dir>, default: ~/.agtrace)    │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐                  │
│  │ projects │  │ sessions │  │log_files │                  │
│  └──────────┘  └──────────┘  └──────────┘                  │
│   (Context)     (Logical     (Physical                       │
│                  Units)       Pointers)                      │
└────────────────┬────────────────────────────────────────────┘
                 │
                 │ (2) Query
                 │     SELECT file paths by session_id
                 │
                 ▼
┌─────────────────────────────────────────────────────────────┐
│                        View Layer                            │
│  (3) Schema-on-Read (Read-path)                             │
│      - Open raw log files                                    │
│      - Parse & normalize to AgentEventV1                     │
│      - Merge/sort by timestamp                               │
│      - Display                                               │
└─────────────────────────────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────────────────┐
│                          User                                │
│              (agtrace list / view output)                    │
└─────────────────────────────────────────────────────────────┘
```

---

## Data Flow

### Phase 1: Scan (Write-path)

**Command:** `agtrace scan`

**Purpose:** Build lightweight index by scanning provider log directories.

**Steps:**
1. **Provider Discovery**
   - Read `<data-dir>/config.toml` (default: `~/.agtrace/config.toml`)
   - Determine `log_root` for each enabled provider

2. **Project Discovery**
   - Resolve `project_root` (via `--project-root`, `AGTRACE_PROJECT_ROOT`, or `cwd`)
   - Calculate `project_hash = sha256(project_root)`

3. **Session Detection**
   - For each provider:
     - Walk `log_root` recursively
     - Detect session files by provider-specific rules:
       - Claude: `*.jsonl` in encoded project directories
       - Codex: `rollout-*.jsonl` in date directories
       - Gemini: `logs.json` + `chats/*.json` in hash directories

4. **Header Parsing (Lightweight)**
   - Open each file
   - Read **first few lines only** (or use streaming parser)
   - Extract:
     - `session_id`
     - `cwd` / `projectHash` (for project matching)
     - `timestamp` (start/end time)
     - `snippet` (e.g., first user message)
   - **Fail-safe:** If parsing fails, log warning but still register pointer

5. **Session Matching**
   - Compare extracted `cwd` / `projectHash` with current `project_root` / `project_hash`
   - If match (or `--all-projects` specified):
     - `INSERT OR REPLACE INTO projects`
     - `INSERT OR REPLACE INTO sessions`
     - `INSERT OR REPLACE INTO log_files`

6. **Update Timestamp**
   - `UPDATE projects SET last_scanned_at = NOW()`

**Performance:**
- Only reads headers (not full files)
- Typical scan: 100 sessions in <1 second
- Incremental re-scan: Check `mod_time`, skip unchanged files

---

### Phase 2: List (Query-path)

**Command:** `agtrace list`

**Purpose:** Display session list from SQLite index.

**Steps:**
1. **Resolve Project Scope**
   - Determine current `project_hash` (or `--all` for all projects)

2. **Query Database**
   ```sql
   SELECT
       id, provider, start_ts, snippet
   FROM sessions
   WHERE
       project_hash = ? AND is_valid = 1
   ORDER BY start_ts DESC
   LIMIT 20;
   ```

3. **Format Output**
   - Plain: Table format with columns (TIME, PROVIDER, ID, PROJECT, SNIPPET)
   - JSON: Array of session objects

**Performance:**
- Query with indexes: <10ms for 1000 sessions
- No file I/O (pure SQLite query)

---

### Phase 3: View (Schema-on-Read)

**Command:** `agtrace view <SESSION_ID>`

**Purpose:** Display session events by dynamically reading and normalizing raw logs.

**Steps:**
1. **Lookup File Paths**
   ```sql
   SELECT path, role FROM log_files WHERE session_id = ?;
   ```

2. **Filter Sidechain Files**
   - Exclude files with `role = "sidechain"` (e.g., Claude's `agent-*.jsonl`)
   - Only process `role = "main"` files for display
   - This prevents internal agent logs from cluttering the output

3. **Open Raw Log Files**
   - For each main `path`:
     - Open file with `BufReader`
     - Iterate line-by-line (JSONL) or parse full JSON

4. **Dynamic Normalization**
   - Apply provider-specific mapper:
     - `ClaudeMapper::normalize(raw) -> Vec<AgentEventV1>`
     - `CodexMapper::normalize(raw) -> Vec<AgentEventV1>`
     - `GeminiMapper::normalize(raw) -> Vec<AgentEventV1>`
   - If parsing fails:
     - Emit `{ event_type: "meta", text: "parse_error: ..." }`
     - Continue (don't crash)

5. **Merge & Sort**
   - Collect events from all files
   - Sort by `ts` (timestamp)
   - Use k-way merge for multiple files (efficient streaming)

6. **Display**
   - `--timeline` (default): Human-readable format
   - `--json`: JSON array of `AgentEventV1`
   - `--raw`: Raw file contents (no normalization)

**Performance:**
- Lazy evaluation: Only parses requested session
- Streaming: Memory usage is O(1) per event, not O(total file size)
- Typical view: 1000 events in <100ms

---

## Key Design Decisions

### 1. Why SQLite?

**Alternatives Considered:**
- File-based JSONL (v1.x): Requires full file walk for listing
- In-memory index: Lost on restart
- External DB (Postgres/MySQL): Overkill for local CLI tool

**SQLite Advantages:**
- Zero-config (embedded)
- Fast indexed queries
- ACID transactions (safe concurrent access)
- Tiny disk footprint (~1-10 MB for 1000 sessions)
- Cross-platform

### 2. Why Schema-on-Read?

**Write-time Normalization (v1.x) Problems:**
- Duplicates data (raw logs + normalized JSONL)
- Slow import (full parsing + normalization)
- Hard to fix normalization bugs (need re-import)

**Schema-on-Read (v2.0) Benefits:**
- No data duplication (raw logs are source of truth)
- Fast scan (header-only parsing)
- Easy to fix/update normalization logic (just re-view, no re-import)
- Fail-safe (parsing errors don't block indexing)

**Trade-offs:**
- View is slower (needs to read/parse raw logs)
  - Mitigation: Cache normalized events (future optimization)
- Requires raw logs to remain available
  - Mitigation: Document this requirement clearly

### 3. Why Pointer-based Architecture?

**Benefits:**
- Respects original logs (no file movement/deletion)
- Supports multi-file sessions (e.g., Claude's `agent-*.jsonl`)
- Enables future features:
  - Watch mode (re-scan on file change)
  - Diff mode (compare raw vs. normalized)
  - Export mode (generate normalized JSONL on-demand)

---

## Component Breakdown

### 1. Scanner Module

**Responsibility:** Provider-specific log file detection and header parsing.

**Key Traits:**
```rust
pub trait Scanner {
    fn scan(&self, log_root: &Path, project_hash: &str) -> Result<Vec<SessionMetadata>>;
}
```

**Implementations:**
- `ClaudeScanner`
- `CodexScanner`
- `GeminiScanner`

**Header Parsing Strategy:**
- Use `serde_json::from_reader` with `#[serde(default)]` to ignore unknown fields
- Read only first N lines (e.g., 10) for performance
- Extract minimal fields:
  - `session_id`
  - `cwd` / `projectHash`
  - `timestamp`
  - `snippet` (first user message if available)

### 2. Storage Module

**Responsibility:** SQLite database abstraction.

**Key Methods:**
```rust
impl Storage {
    pub fn register_session(&self, session: SessionMetadata) -> Result<()>;
    pub fn list_sessions(&self, project_hash: Option<&str>) -> Result<Vec<SessionSummary>>;
    pub fn get_log_files(&self, session_id: &str) -> Result<Vec<LogFile>>;
}
```

**Schema Management:**
- Auto-create tables on first run
- Use `PRAGMA user_version` for schema migrations

### 3. Mapper Module (Normalization)

**Responsibility:** Provider-specific raw log → `AgentEventV1` conversion.

**Key Traits:**
```rust
pub trait Mapper {
    fn normalize(&self, raw: serde_json::Value) -> Result<Vec<AgentEventV1>>;
}
```

**Implementations:**
- `ClaudeMapper` (reuse existing logic from v1.x)
- `CodexMapper` (reuse existing logic from v1.x)
- `GeminiMapper` (reuse existing logic from v1.x)

**Note:** Normalization logic is **unchanged** from v1.x, only the **when** changes (scan-time → view-time).

### 4. View Module

**Responsibility:** Schema-on-Read orchestration.

**Key Methods:**
```rust
impl Viewer {
    pub fn view(&self, session_id: &str, format: ViewFormat) -> Result<()>;
    fn load_and_normalize(&self, file_path: &Path, provider: Source) -> Result<Vec<AgentEventV1>>;
    fn merge_events(&self, events: Vec<Vec<AgentEventV1>>) -> Vec<AgentEventV1>;
}
```

**K-way Merge Algorithm:**
- Use `BinaryHeap` to efficiently merge N sorted streams
- Time complexity: O(N log k) where N = total events, k = number of files

---

## Performance Benchmarks (Target)

| Operation | Target | Notes |
|-----------|--------|-------|
| Scan 100 sessions | <1s | Header-only parsing |
| List 1000 sessions | <10ms | SQLite indexed query |
| View 1 session (1000 events) | <100ms | Full parsing + normalization |
| Re-scan (no changes) | <100ms | Skip unchanged files via `mod_time` |

---

## Migration Path from v1.x

### Option 1: Fresh Start (Recommended)

1. Run `agtrace scan --all-projects` on existing providers
2. Old data in `<data-dir>/projects/` can be archived or deleted
3. New SQLite DB references original raw logs

### Option 2: Gradual Migration

1. Keep v1.x data intact
2. Run `agtrace scan` to build new index
3. Both old (JSONL-based) and new (pointer-based) views coexist
4. Deprecate old commands in v3.0

---

## Future Optimizations

### 1. Caching Layer

Cache normalized events in SQLite `events_cache` table:
- First `view`: Parse and cache
- Subsequent `view`: Read from cache (fast)
- Invalidate cache on `mod_time` change

### 2. Incremental Scan

Track `scan_cache` table:
- Store `(file_path, last_mod_time, last_scanned_at)`
- Skip unchanged files on re-scan

### 3. Watch Mode

Monitor log directories with `notify` crate:
```sh
agtrace watch
# Auto-scan when new log files appear
```

### 4. Parallel Scanning

Scan multiple providers in parallel:
```rust
tokio::join!(
    scan_claude(),
    scan_codex(),
    scan_gemini()
)
```

---

This architecture provides a solid foundation for the Pointer Edition, balancing performance, maintainability, and future extensibility.
