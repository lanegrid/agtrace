# agtrace Database Schema

## Overview

agtrace v2.0 uses SQLite as a lightweight index for agent session metadata. The database stores **pointers** to raw log files, not the normalized events themselves.

**Key principle:** Schema-on-Read
- Write-time: Store minimal metadata (file paths, timestamps, project context)
- Read-time: Dynamically load and normalize raw logs

**Database location:** `<data-dir>/agtrace.db` (default: `~/.agtrace/agtrace.db`)

---

## Schema Definition

### 1. `projects` Table

**Purpose:** Tracks projects (context anchor) to enable cross-provider session grouping.

```sql
CREATE TABLE projects (
    hash            TEXT PRIMARY KEY,  -- sha256(project_root).hex (64 chars)
    root_path       TEXT,              -- Absolute path (can be NULL for Gemini)
    last_scanned_at TEXT               -- RFC3339 timestamp
);
```

**Fields:**
- `hash`: Project identifier (sha256 of canonical project root path)
- `root_path`: Absolute path to project root. Can be NULL if:
  - Gemini sessions detected by hash before path is known
  - Project root is not yet resolved
- `last_scanned_at`: Last time this project was scanned (updated by `agtrace scan`)

**Notes:**
- Gemini logs contain `projectHash` but not `projectRoot`
- Claude/Codex logs contain `cwd` which resolves to `project_root`
- Multiple providers can share the same project via `hash`

---

### 2. `sessions` Table

**Purpose:** Logical unit of work (conversation/execution). Primary listing unit.

```sql
CREATE TABLE sessions (
    id              TEXT PRIMARY KEY,  -- UUID from provider
    project_hash    TEXT NOT NULL,     -- FK to projects.hash
    provider        TEXT NOT NULL,     -- 'claude', 'codex', 'gemini'

    start_ts        TEXT,              -- ISO8601 start time
    end_ts          TEXT,              -- ISO8601 end/last update time

    snippet         TEXT,              -- Summary for list view (e.g., first prompt)
    is_valid        BOOLEAN DEFAULT 1, -- Soft delete flag

    FOREIGN KEY (project_hash) REFERENCES projects(hash)
);
```

**Fields:**
- `id`: Session identifier (UUID from provider logs)
- `project_hash`: Links session to project
- `provider`: Source provider (`claude`, `codex`, `gemini`)
- `start_ts` / `end_ts`: Session time range
- `snippet`: Short summary for `agtrace list` output (e.g., first user message)
- `is_valid`: Soft delete flag (instead of hard deletion)

**Indexes:**
```sql
CREATE INDEX idx_sessions_project ON sessions(project_hash);
CREATE INDEX idx_sessions_ts ON sessions(start_ts DESC);
```

---

### 3. `log_files` Table

**Purpose:** Physical file pointers. Maps session to actual log files.

```sql
CREATE TABLE log_files (
    path            TEXT PRIMARY KEY,  -- Absolute file path
    session_id      TEXT NOT NULL,     -- FK to sessions.id

    role            TEXT NOT NULL,     -- 'main', 'sidechain', 'meta'
    file_size       INTEGER,           -- For change detection
    mod_time        TEXT,              -- For change detection

    FOREIGN KEY (session_id) REFERENCES sessions(id)
);
```

**Fields:**
- `path`: Absolute path to raw log file
- `session_id`: Links file to session
- `role`: File type classification:
  - `main`: Primary session log (e.g., `session-id.jsonl` for Claude)
  - `sidechain`: Additional logs (e.g., `agent-*.jsonl` for Claude)
  - `meta`: Metadata files (e.g., `logs.json` for Gemini)
- `file_size` / `mod_time`: Change detection (for incremental re-scan)

**Indexes:**
```sql
CREATE INDEX idx_files_session ON log_files(session_id);
```

---

## Schema Initialization

```sql
-- Create tables
CREATE TABLE IF NOT EXISTS projects (
    hash TEXT PRIMARY KEY,
    root_path TEXT,
    last_scanned_at TEXT
);

CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    project_hash TEXT NOT NULL,
    provider TEXT NOT NULL,
    start_ts TEXT,
    end_ts TEXT,
    snippet TEXT,
    is_valid BOOLEAN DEFAULT 1,
    FOREIGN KEY (project_hash) REFERENCES projects(hash)
);

CREATE TABLE IF NOT EXISTS log_files (
    path TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    role TEXT NOT NULL,
    file_size INTEGER,
    mod_time TEXT,
    FOREIGN KEY (session_id) REFERENCES sessions(id)
);

-- Create indexes
CREATE INDEX IF NOT EXISTS idx_sessions_project ON sessions(project_hash);
CREATE INDEX IF NOT EXISTS idx_sessions_ts ON sessions(start_ts DESC);
CREATE INDEX IF NOT EXISTS idx_files_session ON log_files(session_id);
```

---

## Data Flow

### 1. Scan Phase (`agtrace scan`)

```
1. Determine project_root → Calculate project_hash
2. For each provider:
   a. Scan log_root for session files
   b. Read header (session_id, timestamps, project info)
   c. Match session to current project
   d. INSERT/UPDATE projects table
   e. INSERT/UPDATE sessions table
   f. INSERT/UPDATE log_files table
3. Update projects.last_scanned_at
```

### 2. List Phase (`agtrace list`)

```sql
-- List sessions for current project
SELECT
    sessions.id,
    sessions.provider,
    sessions.start_ts,
    sessions.snippet
FROM sessions
WHERE
    sessions.project_hash = ?
    AND sessions.is_valid = 1
ORDER BY sessions.start_ts DESC
LIMIT 20;
```

### 3. View Phase (`agtrace view <SESSION_ID>`)

```sql
-- Get all log files for session
SELECT path, role
FROM log_files
WHERE session_id = ?
ORDER BY role;  -- Ensure 'main' files are read first
```

Then:
1. Open each file
2. Parse and normalize to `AgentEventV1` (Schema-on-Read)
3. Merge and sort by timestamp
4. Display

---

## Migration Strategy

### From v1.x (File-based) to v2.0 (SQLite)

**Option 1: Clean Migration**
- Run `agtrace scan --all-projects` on existing providers
- Old data in `<data-dir>/projects/<hash>/sessions/*.jsonl` can be archived
- New DB will reference original raw logs

**Option 2: Import Existing Data**
- Write migration script to:
  1. Read existing JSONL files
  2. Extract metadata (session_id, project_hash, timestamps)
  3. Populate SQLite tables
  4. Keep JSONL files as-is for backward compatibility

---

## Database Maintenance

### Rebuild Index
```sh
agtrace scan --force --all-projects
```

### Soft Delete Session
```sql
UPDATE sessions SET is_valid = 0 WHERE id = '<session-id>';
```

### Clean Invalid Sessions
```sql
DELETE FROM log_files WHERE session_id IN (
    SELECT id FROM sessions WHERE is_valid = 0
);
DELETE FROM sessions WHERE is_valid = 0;
```

### Vacuum (Compact Database)
```sql
VACUUM;
```

---

## Performance Considerations

### Indexes
- `idx_sessions_project`: Fast filtering by project
- `idx_sessions_ts`: Fast chronological sorting
- `idx_files_session`: Fast file lookup per session

### Query Optimization
- Use `LIMIT` for large result sets
- Index all FK columns
- Consider `PRAGMA journal_mode=WAL` for concurrent reads

### Scaling
- Expected DB size: ~1-10 MB for 1000 sessions
- SQLite handles 100K+ sessions efficiently
- Schema-on-Read means DB size stays small (only metadata)

---

## Future Extensions

### Potential New Tables

**`events_index` (Full-text search)**
```sql
CREATE VIRTUAL TABLE events_index USING fts5(
    session_id,
    event_id,
    text,
    content='sessions'
);
```

**`session_stats` (Pre-computed statistics)**
```sql
CREATE TABLE session_stats (
    session_id TEXT PRIMARY KEY,
    event_count INTEGER,
    token_count INTEGER,
    tool_call_count INTEGER,
    FOREIGN KEY (session_id) REFERENCES sessions(id)
);
```

**`scan_cache` (Incremental scan optimization)**
```sql
CREATE TABLE scan_cache (
    file_path TEXT PRIMARY KEY,
    session_id TEXT,
    project_hash TEXT,
    last_mod_time TEXT,
    last_scanned_at TEXT
);
```

---

This schema provides a lightweight, scalable foundation for the Pointer Edition architecture.
