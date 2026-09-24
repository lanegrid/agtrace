use rusqlite::Connection;

use crate::Result;

// Schema version (increment when changing table definitions)
pub const SCHEMA_VERSION: i32 = 7;

// NOTE: Database Design Rationale (Pointer Edition)
//
// Why Schema-on-Read (not Schema-on-Write)?
// - Provider logs change format frequently; parsing logic needs flexibility
// - Event normalization is complex (Codex dedup, Claude usage extraction, etc.)
// - Raw logs are source of truth; DB is just an index for fast lookup
// - Keeps DB lightweight and migration-free when schema evolves
//
// Why hash-based project identification?
// - Hash allows cross-provider session grouping before path resolution
// - Enables "same project" detection across Claude/Codex
//
// Why one session row per agent-file owner (v7)?
// - Claude main / teammate transcripts and every Codex thread own their file; Claude
//   subagents / forks live under their parent session (log_files.role = subagent|fork)
// - Rows are built from file headers only (no full parse); parent_session_id has no FK
//   because children may be indexed before their parent
// - The index is a pointer store: links unknown at index time (e.g. teammates without a
//   team config) are resolved live by the agent graph, not here
//
// Why soft delete (is_valid flag)?
// - Avoid orphaned log_files entries when session is deleted
// - Enable "undo" or audit trail without complex cascade logic
// - Simplifies cleanup: UPDATE instead of multi-table DELETE transaction

pub fn init_schema(conn: &Connection) -> Result<()> {
    let current_version: i32 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;

    if current_version != SCHEMA_VERSION {
        drop_all_tables(conn)?;
    }

    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS projects (
            hash TEXT PRIMARY KEY,
            root_path TEXT,
            last_scanned_at TEXT
        );

        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            project_hash TEXT NOT NULL,
            repository_hash TEXT,
            provider TEXT NOT NULL,
            start_ts TEXT,
            end_ts TEXT,
            snippet TEXT,
            is_valid BOOLEAN DEFAULT 1,
            agent_kind TEXT NOT NULL,
            agent_name TEXT,
            agent_path TEXT,
            team_name TEXT,
            root_session_id TEXT,
            parent_session_id TEXT,
            spawn_call_id TEXT,
            FOREIGN KEY (project_hash) REFERENCES projects(hash)
        );

        CREATE TABLE IF NOT EXISTS log_files (
            path TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            role TEXT NOT NULL,
            agent_id TEXT NOT NULL,
            agent_name TEXT,
            spawn_call_id TEXT,
            file_size INTEGER,
            mod_time TEXT,
            FOREIGN KEY (session_id) REFERENCES sessions(id)
        );

        CREATE INDEX IF NOT EXISTS idx_sessions_project ON sessions(project_hash);
        CREATE INDEX IF NOT EXISTS idx_sessions_repository ON sessions(repository_hash);
        CREATE INDEX IF NOT EXISTS idx_sessions_ts ON sessions(start_ts DESC);
        CREATE INDEX IF NOT EXISTS idx_sessions_parent ON sessions(parent_session_id);
        CREATE INDEX IF NOT EXISTS idx_sessions_root ON sessions(root_session_id);
        CREATE INDEX IF NOT EXISTS idx_files_session ON log_files(session_id);
        "#,
    )?;

    conn.execute(&format!("PRAGMA user_version = {}", SCHEMA_VERSION), [])?;

    Ok(())
}

fn drop_all_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        DROP TABLE IF EXISTS log_files;
        DROP TABLE IF EXISTS sessions;
        DROP TABLE IF EXISTS projects;
        "#,
    )?;
    Ok(())
}
