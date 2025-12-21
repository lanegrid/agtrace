use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

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

#[derive(Debug, Clone)]
pub struct ProjectRecord {
    pub hash: String,
    pub root_path: Option<String>,
    pub last_scanned_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SessionRecord {
    pub id: String,
    pub project_hash: String,
    pub provider: String,
    pub start_ts: Option<String>,
    pub end_ts: Option<String>,
    pub snippet: Option<String>,
    pub is_valid: bool,
}

#[derive(Debug, Clone)]
pub struct LogFileRecord {
    pub path: String,
    pub session_id: String,
    pub role: String,
    pub file_size: Option<i64>,
    pub mod_time: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub provider: String,
    pub project_hash: String,
    pub start_ts: Option<String>,
    pub snippet: Option<String>,
}

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)
            .with_context(|| format!("Failed to open database: {}", db_path.display()))?;

        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    pub fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
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

        Ok(())
    }

    pub fn insert_or_update_project(&self, project: &ProjectRecord) -> Result<()> {
        self.conn.execute(
            r#"
            INSERT INTO projects (hash, root_path, last_scanned_at)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(hash) DO UPDATE SET
                root_path = COALESCE(?2, root_path),
                last_scanned_at = ?3
            "#,
            params![&project.hash, &project.root_path, &project.last_scanned_at],
        )?;

        Ok(())
    }

    pub fn insert_or_update_session(&self, session: &SessionRecord) -> Result<()> {
        self.conn.execute(
            r#"
            INSERT INTO sessions (id, project_hash, provider, start_ts, end_ts, snippet, is_valid)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(id) DO UPDATE SET
                project_hash = ?2,
                provider = ?3,
                start_ts = COALESCE(?4, start_ts),
                end_ts = COALESCE(?5, end_ts),
                snippet = COALESCE(?6, snippet),
                is_valid = ?7
            "#,
            params![
                &session.id,
                &session.project_hash,
                &session.provider,
                &session.start_ts,
                &session.end_ts,
                &session.snippet,
                &session.is_valid
            ],
        )?;

        Ok(())
    }

    pub fn insert_or_update_log_file(&self, log_file: &LogFileRecord) -> Result<()> {
        self.conn.execute(
            r#"
            INSERT INTO log_files (path, session_id, role, file_size, mod_time)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(path) DO UPDATE SET
                session_id = ?2,
                role = ?3,
                file_size = ?4,
                mod_time = ?5
            "#,
            params![
                &log_file.path,
                &log_file.session_id,
                &log_file.role,
                &log_file.file_size,
                &log_file.mod_time
            ],
        )?;

        Ok(())
    }

    pub fn get_session_by_id(&self, session_id: &str) -> Result<Option<SessionSummary>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, provider, project_hash, start_ts, snippet
            FROM sessions
            WHERE id = ?1 AND is_valid = 1
            "#,
        )?;

        let mut rows = stmt.query([session_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(SessionSummary {
                id: row.get(0)?,
                provider: row.get(1)?,
                project_hash: row.get(2)?,
                start_ts: row.get(3)?,
                snippet: row.get(4)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn list_sessions(
        &self,
        project_hash: Option<&str>,
        limit: usize,
    ) -> Result<Vec<SessionSummary>> {
        let query = if let Some(hash) = project_hash {
            format!(
                r#"
                SELECT id, provider, project_hash, start_ts, snippet
                FROM sessions
                WHERE project_hash = '{}' AND is_valid = 1
                ORDER BY start_ts DESC
                LIMIT {}
                "#,
                hash, limit
            )
        } else {
            format!(
                r#"
                SELECT id, provider, project_hash, start_ts, snippet
                FROM sessions
                WHERE is_valid = 1
                ORDER BY start_ts DESC
                LIMIT {}
                "#,
                limit
            )
        };

        let mut stmt = self.conn.prepare(&query)?;
        let sessions = stmt
            .query_map([], |row| {
                Ok(SessionSummary {
                    id: row.get(0)?,
                    provider: row.get(1)?,
                    project_hash: row.get(2)?,
                    start_ts: row.get(3)?,
                    snippet: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(sessions)
    }

    pub fn get_session_files(&self, session_id: &str) -> Result<Vec<LogFileRecord>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT path, session_id, role, file_size, mod_time
            FROM log_files
            WHERE session_id = ?1
            ORDER BY role
            "#,
        )?;

        let files = stmt
            .query_map([session_id], |row| {
                Ok(LogFileRecord {
                    path: row.get(0)?,
                    session_id: row.get(1)?,
                    role: row.get(2)?,
                    file_size: row.get(3)?,
                    mod_time: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(files)
    }

    /// Find session by ID prefix (supports short IDs like "7f2abd2d")
    pub fn find_session_by_prefix(&self, prefix: &str) -> Result<Option<String>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id
            FROM sessions
            WHERE id LIKE ?1
            LIMIT 2
            "#,
        )?;

        let pattern = format!("{}%", prefix);
        let mut matches: Vec<String> = stmt
            .query_map([&pattern], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        match matches.len() {
            0 => Ok(None),
            1 => Ok(Some(matches.remove(0))),
            _ => anyhow::bail!(
                "Ambiguous session ID prefix '{}': multiple sessions match",
                prefix
            ),
        }
    }

    pub fn get_project(&self, hash: &str) -> Result<Option<ProjectRecord>> {
        let result = self
            .conn
            .query_row(
                r#"
            SELECT hash, root_path, last_scanned_at
            FROM projects
            WHERE hash = ?1
            "#,
                [hash],
                |row| {
                    Ok(ProjectRecord {
                        hash: row.get(0)?,
                        root_path: row.get(1)?,
                        last_scanned_at: row.get(2)?,
                    })
                },
            )
            .optional()?;

        Ok(result)
    }

    pub fn list_projects(&self) -> Result<Vec<ProjectRecord>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT hash, root_path, last_scanned_at
            FROM projects
            ORDER BY last_scanned_at DESC
            "#,
        )?;

        let projects = stmt
            .query_map([], |row| {
                Ok(ProjectRecord {
                    hash: row.get(0)?,
                    root_path: row.get(1)?,
                    last_scanned_at: row.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(projects)
    }

    pub fn count_sessions_for_project(&self, project_hash: &str) -> Result<usize> {
        let count: i64 = self.conn.query_row(
            r#"
            SELECT COUNT(*)
            FROM sessions
            WHERE project_hash = ?1 AND is_valid = 1
            "#,
            [project_hash],
            |row| row.get(0),
        )?;

        Ok(count as usize)
    }

    pub fn vacuum(&self) -> Result<()> {
        self.conn.execute("VACUUM", [])?;
        println!("Database vacuumed successfully");
        Ok(())
    }

    pub fn get_all_log_files(&self) -> Result<Vec<LogFileRecord>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT path, session_id, role, file_size, mod_time
            FROM log_files
            ORDER BY path
            "#,
        )?;

        let files = stmt
            .query_map([], |row| {
                Ok(LogFileRecord {
                    path: row.get(0)?,
                    session_id: row.get(1)?,
                    role: row.get(2)?,
                    file_size: row.get(3)?,
                    mod_time: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_initialization() {
        let db = Database::open_in_memory().unwrap();

        let projects = db.list_projects().unwrap();
        assert_eq!(projects.len(), 0);
    }

    #[test]
    fn test_insert_project() {
        let db = Database::open_in_memory().unwrap();

        let project = ProjectRecord {
            hash: "abc123".to_string(),
            root_path: Some("/path/to/project".to_string()),
            last_scanned_at: Some("2025-12-10T10:00:00Z".to_string()),
        };

        db.insert_or_update_project(&project).unwrap();

        let retrieved = db.get_project("abc123").unwrap().unwrap();
        assert_eq!(retrieved.hash, "abc123");
        assert_eq!(retrieved.root_path, Some("/path/to/project".to_string()));
    }

    #[test]
    fn test_insert_session_with_fk() {
        let db = Database::open_in_memory().unwrap();

        let project = ProjectRecord {
            hash: "abc123".to_string(),
            root_path: Some("/path/to/project".to_string()),
            last_scanned_at: Some("2025-12-10T10:00:00Z".to_string()),
        };
        db.insert_or_update_project(&project).unwrap();

        let session = SessionRecord {
            id: "session-001".to_string(),
            project_hash: "abc123".to_string(),
            provider: "claude".to_string(),
            start_ts: Some("2025-12-10T10:05:00Z".to_string()),
            end_ts: Some("2025-12-10T10:15:00Z".to_string()),
            snippet: Some("Test session".to_string()),
            is_valid: true,
        };

        db.insert_or_update_session(&session).unwrap();

        let sessions = db.list_sessions(Some("abc123"), 10).unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, "session-001");
        assert_eq!(sessions[0].provider, "claude");
    }

    #[test]
    fn test_insert_log_file() {
        let db = Database::open_in_memory().unwrap();

        let project = ProjectRecord {
            hash: "abc123".to_string(),
            root_path: Some("/path/to/project".to_string()),
            last_scanned_at: Some("2025-12-10T10:00:00Z".to_string()),
        };
        db.insert_or_update_project(&project).unwrap();

        let session = SessionRecord {
            id: "session-001".to_string(),
            project_hash: "abc123".to_string(),
            provider: "claude".to_string(),
            start_ts: Some("2025-12-10T10:05:00Z".to_string()),
            end_ts: None,
            snippet: None,
            is_valid: true,
        };
        db.insert_or_update_session(&session).unwrap();

        let log_file = LogFileRecord {
            path: "/path/to/log.jsonl".to_string(),
            session_id: "session-001".to_string(),
            role: "main".to_string(),
            file_size: Some(1024),
            mod_time: Some("2025-12-10T10:05:00Z".to_string()),
        };

        db.insert_or_update_log_file(&log_file).unwrap();

        let files = db.get_session_files("session-001").unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "/path/to/log.jsonl");
        assert_eq!(files[0].role, "main");
    }

    #[test]
    fn test_list_sessions_query() {
        let db = Database::open_in_memory().unwrap();

        let project = ProjectRecord {
            hash: "abc123".to_string(),
            root_path: Some("/path/to/project".to_string()),
            last_scanned_at: Some("2025-12-10T10:00:00Z".to_string()),
        };
        db.insert_or_update_project(&project).unwrap();

        for i in 1..=5 {
            let session = SessionRecord {
                id: format!("session-{:03}", i),
                project_hash: "abc123".to_string(),
                provider: "claude".to_string(),
                start_ts: Some(format!("2025-12-10T10:{:02}:00Z", i)),
                end_ts: None,
                snippet: Some(format!("Session {}", i)),
                is_valid: true,
            };
            db.insert_or_update_session(&session).unwrap();
        }

        let sessions = db.list_sessions(Some("abc123"), 10).unwrap();
        assert_eq!(sessions.len(), 5);

        let sessions_limited = db.list_sessions(Some("abc123"), 3).unwrap();
        assert_eq!(sessions_limited.len(), 3);
    }

    #[test]
    fn test_count_sessions_for_project() {
        let db = Database::open_in_memory().unwrap();

        let project = ProjectRecord {
            hash: "abc123".to_string(),
            root_path: Some("/path/to/project".to_string()),
            last_scanned_at: Some("2025-12-10T10:00:00Z".to_string()),
        };
        db.insert_or_update_project(&project).unwrap();

        for i in 1..=3 {
            let session = SessionRecord {
                id: format!("session-{:03}", i),
                project_hash: "abc123".to_string(),
                provider: "claude".to_string(),
                start_ts: Some(format!("2025-12-10T10:{:02}:00Z", i)),
                end_ts: None,
                snippet: None,
                is_valid: true,
            };
            db.insert_or_update_session(&session).unwrap();
        }

        let count = db.count_sessions_for_project("abc123").unwrap();
        assert_eq!(count, 3);
    }
}
