use rusqlite::Connection;

use crate::Result;

// Schema version (increment when changing table definitions)
pub const SCHEMA_VERSION: i32 = 2;

// NOTE: Database Design Rationale (Pointer Edition)
//
// Why Schema-on-Read (not Schema-on-Write)?
// - Provider logs change format frequently; parsing logic needs flexibility
// - Event normalization is complex (Gemini unfold, Codex dedup, etc.)
// - Raw logs are source of truth; DB is just an index for fast lookup
// - Keeps DB lightweight and migration-free when schema evolves
//
// Why hash-based project identification?
// - Gemini logs contain projectHash but not projectRoot path
// - Hash allows cross-provider session grouping before path resolution
// - Enables "same project" detection across Claude/Codex/Gemini
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

        CREATE INDEX IF NOT EXISTS idx_sessions_project ON sessions(project_hash);
        CREATE INDEX IF NOT EXISTS idx_sessions_ts ON sessions(start_ts DESC);
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
