use rusqlite::Connection;
use std::path::Path;

use crate::{Result, queries, records::*, schema};

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        let db = Self { conn };
        schema::init_schema(&db.conn)?;
        Ok(db)
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        schema::init_schema(&db.conn)?;
        Ok(db)
    }

    // Project operations
    pub fn insert_or_update_project(&self, project: &ProjectRecord) -> Result<()> {
        queries::project::insert_or_update(&self.conn, project)
    }

    pub fn get_project(&self, hash: &str) -> Result<Option<ProjectRecord>> {
        queries::project::get(&self.conn, hash)
    }

    pub fn list_projects(&self) -> Result<Vec<ProjectRecord>> {
        queries::project::list(&self.conn)
    }

    pub fn count_sessions_for_project(&self, project_hash: &str) -> Result<usize> {
        queries::project::count_sessions(&self.conn, project_hash)
    }

    // Session operations
    pub fn insert_or_update_session(&self, session: &SessionRecord) -> Result<()> {
        queries::session::insert_or_update(&self.conn, session)
    }

    pub fn get_session_by_id(&self, session_id: &str) -> Result<Option<SessionSummary>> {
        queries::session::get_by_id(&self.conn, session_id)
    }

    pub fn list_sessions(
        &self,
        project_hash: Option<&agtrace_types::ProjectHash>,
        provider: Option<&str>,
        order: agtrace_types::SessionOrder,
        limit: Option<usize>,
        top_level_only: bool,
    ) -> Result<Vec<SessionSummary>> {
        queries::session::list(
            &self.conn,
            project_hash,
            provider,
            order,
            limit,
            top_level_only,
        )
    }

    pub fn find_session_by_prefix(&self, prefix: &str) -> Result<Option<String>> {
        queries::session::find_by_prefix(&self.conn, prefix)
    }

    pub fn get_child_sessions(&self, parent_session_id: &str) -> Result<Vec<SessionSummary>> {
        queries::session::get_children(&self.conn, parent_session_id)
    }

    // Log file operations
    pub fn insert_or_update_log_file(&self, log_file: &LogFileRecord) -> Result<()> {
        queries::log_file::insert_or_update(&self.conn, log_file)
    }

    pub fn get_session_files(&self, session_id: &str) -> Result<Vec<LogFileRecord>> {
        queries::log_file::get_session_files(&self.conn, session_id)
    }

    pub fn get_all_log_files(&self) -> Result<Vec<LogFileRecord>> {
        queries::log_file::get_all(&self.conn)
    }

    // Utility operations
    pub fn vacuum(&self) -> Result<()> {
        self.conn.execute("VACUUM", [])?;
        println!("Database vacuumed successfully");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::SCHEMA_VERSION;
    use rusqlite::params;

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
            hash: agtrace_types::ProjectHash::from("abc123"),
            root_path: Some("/path/to/project".to_string()),
            last_scanned_at: Some("2025-12-10T10:00:00Z".to_string()),
        };

        db.insert_or_update_project(&project).unwrap();

        let retrieved = db.get_project("abc123").unwrap().unwrap();
        assert_eq!(retrieved.hash, agtrace_types::ProjectHash::from("abc123"));
        assert_eq!(retrieved.root_path, Some("/path/to/project".to_string()));
    }

    #[test]
    fn test_insert_session_with_fk() {
        let db = Database::open_in_memory().unwrap();

        let project = ProjectRecord {
            hash: agtrace_types::ProjectHash::from("abc123"),
            root_path: Some("/path/to/project".to_string()),
            last_scanned_at: Some("2025-12-10T10:00:00Z".to_string()),
        };
        db.insert_or_update_project(&project).unwrap();

        let session = SessionRecord {
            id: "session-001".to_string(),
            project_hash: agtrace_types::ProjectHash::from("abc123"),
            provider: "claude".to_string(),
            start_ts: Some("2025-12-10T10:05:00Z".to_string()),
            end_ts: Some("2025-12-10T10:15:00Z".to_string()),
            snippet: Some("Test session".to_string()),
            is_valid: true,
            parent_session_id: None,
            spawned_by: None,
        };

        db.insert_or_update_session(&session).unwrap();

        let sessions = db
            .list_sessions(
                Some(&agtrace_types::ProjectHash::from("abc123")),
                None,
                agtrace_types::SessionOrder::default(),
                Some(10),
                false,
            )
            .unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, "session-001");
        assert_eq!(sessions[0].provider, "claude");
    }

    #[test]
    fn test_insert_log_file() {
        let db = Database::open_in_memory().unwrap();

        let project = ProjectRecord {
            hash: agtrace_types::ProjectHash::from("abc123"),
            root_path: Some("/path/to/project".to_string()),
            last_scanned_at: Some("2025-12-10T10:00:00Z".to_string()),
        };
        db.insert_or_update_project(&project).unwrap();

        let session = SessionRecord {
            id: "session-001".to_string(),
            project_hash: agtrace_types::ProjectHash::from("abc123"),
            provider: "claude".to_string(),
            start_ts: Some("2025-12-10T10:05:00Z".to_string()),
            end_ts: None,
            snippet: None,
            is_valid: true,
            parent_session_id: None,
            spawned_by: None,
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
            hash: agtrace_types::ProjectHash::from("abc123"),
            root_path: Some("/path/to/project".to_string()),
            last_scanned_at: Some("2025-12-10T10:00:00Z".to_string()),
        };
        db.insert_or_update_project(&project).unwrap();

        for i in 1..=5 {
            let session = SessionRecord {
                id: format!("session-{:03}", i),
                project_hash: agtrace_types::ProjectHash::from("abc123"),
                provider: "claude".to_string(),
                start_ts: Some(format!("2025-12-10T10:{:02}:00Z", i)),
                end_ts: None,
                snippet: Some(format!("Session {}", i)),
                is_valid: true,
                parent_session_id: None,
                spawned_by: None,
            };
            db.insert_or_update_session(&session).unwrap();
        }

        let sessions = db
            .list_sessions(
                Some(&agtrace_types::ProjectHash::from("abc123")),
                None,
                agtrace_types::SessionOrder::default(),
                Some(10),
                false,
            )
            .unwrap();
        assert_eq!(sessions.len(), 5);

        let sessions_limited = db
            .list_sessions(
                Some(&agtrace_types::ProjectHash::from("abc123")),
                None,
                agtrace_types::SessionOrder::default(),
                Some(3),
                false,
            )
            .unwrap();
        assert_eq!(sessions_limited.len(), 3);
    }

    #[test]
    fn test_count_sessions_for_project() {
        let db = Database::open_in_memory().unwrap();

        let project = ProjectRecord {
            hash: agtrace_types::ProjectHash::from("abc123"),
            root_path: Some("/path/to/project".to_string()),
            last_scanned_at: Some("2025-12-10T10:00:00Z".to_string()),
        };
        db.insert_or_update_project(&project).unwrap();

        for i in 1..=3 {
            let session = SessionRecord {
                id: format!("session-{:03}", i),
                project_hash: agtrace_types::ProjectHash::from("abc123"),
                provider: "claude".to_string(),
                start_ts: Some(format!("2025-12-10T10:{:02}:00Z", i)),
                end_ts: None,
                snippet: None,
                is_valid: true,
                parent_session_id: None,
                spawned_by: None,
            };
            db.insert_or_update_session(&session).unwrap();
        }

        let count = db.count_sessions_for_project("abc123").unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn test_schema_version_set_on_init() {
        let db = Database::open_in_memory().unwrap();

        let version: i32 = db
            .conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();

        assert_eq!(version, SCHEMA_VERSION);
    }

    #[test]
    fn test_schema_rebuild_on_version_mismatch() {
        let conn = Connection::open_in_memory().unwrap();

        conn.execute_batch(
            r#"
            CREATE TABLE projects (hash TEXT PRIMARY KEY);
            CREATE TABLE sessions (id TEXT PRIMARY KEY);
            PRAGMA user_version = 999;
            "#,
        )
        .unwrap();

        conn.execute(
            "INSERT INTO projects (hash) VALUES (?1)",
            params!["old_data"],
        )
        .unwrap();

        let db = Database { conn };
        schema::init_schema(&db.conn).unwrap();

        let version: i32 = db
            .conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);

        let count: i64 = db
            .conn
            .query_row("SELECT COUNT(*) FROM projects", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_schema_preserved_on_version_match() {
        let db = Database::open_in_memory().unwrap();

        let project = ProjectRecord {
            hash: agtrace_types::ProjectHash::from("abc123"),
            root_path: Some("/path/to/project".to_string()),
            last_scanned_at: Some("2025-12-10T10:00:00Z".to_string()),
        };
        db.insert_or_update_project(&project).unwrap();

        schema::init_schema(&db.conn).unwrap();

        let retrieved = db.get_project("abc123").unwrap();
        assert!(retrieved.is_some());
        assert_eq!(
            retrieved.unwrap().hash,
            agtrace_types::ProjectHash::from("abc123")
        );
    }
}
