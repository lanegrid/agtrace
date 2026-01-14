//! Integration tests for schema migration
//!
//! These tests verify that Database::open correctly handles schema version mismatches
//! by dropping and recreating tables.

use agtrace_index::{Database, ProjectRecord, SessionRecord};
use agtrace_types::{ProjectHash, SessionOrder, SpawnContext};
use rusqlite::Connection;
use std::path::Path;
use tempfile::TempDir;

/// Create a database with old schema (version 2) that lacks parent_session_id columns
fn create_old_schema_db(path: &Path) {
    let conn = Connection::open(path).unwrap();

    conn.execute_batch(
        r#"
        CREATE TABLE projects (
            hash TEXT PRIMARY KEY,
            root_path TEXT,
            last_scanned_at TEXT
        );

        CREATE TABLE sessions (
            id TEXT PRIMARY KEY,
            project_hash TEXT NOT NULL,
            provider TEXT NOT NULL,
            start_ts TEXT,
            end_ts TEXT,
            snippet TEXT,
            is_valid BOOLEAN DEFAULT 1,
            FOREIGN KEY (project_hash) REFERENCES projects(hash)
        );

        CREATE TABLE log_files (
            path TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            role TEXT NOT NULL,
            file_size INTEGER,
            mod_time TEXT,
            FOREIGN KEY (session_id) REFERENCES sessions(id)
        );

        CREATE INDEX idx_sessions_project ON sessions(project_hash);
        CREATE INDEX idx_sessions_ts ON sessions(start_ts DESC);
        CREATE INDEX idx_files_session ON log_files(session_id);

        PRAGMA user_version = 2;
        "#,
    )
    .unwrap();

    // Insert some test data
    conn.execute(
        "INSERT INTO projects (hash, root_path) VALUES ('test_hash', '/test/path')",
        [],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO sessions (id, project_hash, provider, start_ts, is_valid)
         VALUES ('old_session', 'test_hash', 'claude_code', '2024-01-01T00:00:00Z', 1)",
        [],
    )
    .unwrap();
}

#[test]
fn test_auto_migration_from_old_schema() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    // Create old schema DB (version 2, without parent_session_id)
    create_old_schema_db(&db_path);

    // Verify old schema version
    {
        let conn = Connection::open(&db_path).unwrap();
        let version: i32 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 2, "Should start with schema version 2");
    }

    // Open with Database::open - should auto-migrate
    let db = Database::open(&db_path).expect("Database::open should succeed and auto-migrate");

    // Verify schema was upgraded
    {
        let conn = Connection::open(&db_path).unwrap();
        let version: i32 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 6, "Schema should be upgraded to version 6");
    }

    // Verify new columns work - insert session with parent_session_id
    let project = ProjectRecord {
        hash: ProjectHash::from("new_hash"),
        root_path: Some("/new/path".to_string()),
        last_scanned_at: None,
    };
    db.insert_or_update_project(&project).unwrap();

    let parent_session = SessionRecord {
        id: "parent_session".to_string(),
        project_hash: ProjectHash::from("new_hash"),
        repository_hash: None,
        provider: "claude_code".to_string(),
        start_ts: Some("2024-01-01T00:00:00Z".to_string()),
        end_ts: None,
        snippet: Some("parent".to_string()),
        is_valid: true,
        parent_session_id: None,
        spawned_by: None,
    };
    db.insert_or_update_session(&parent_session).unwrap();

    let child_session = SessionRecord {
        id: "child_session".to_string(),
        project_hash: ProjectHash::from("new_hash"),
        repository_hash: None,
        provider: "claude_code".to_string(),
        start_ts: Some("2024-01-01T01:00:00Z".to_string()),
        end_ts: None,
        snippet: Some("child".to_string()),
        is_valid: true,
        parent_session_id: Some("parent_session".to_string()),
        spawned_by: Some(SpawnContext {
            turn_index: 1,
            step_index: 2,
        }),
    };
    db.insert_or_update_session(&child_session).unwrap();

    // Query using new columns
    let children = db.get_child_sessions("parent_session").unwrap();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].id, "child_session");
    assert_eq!(
        children[0].parent_session_id,
        Some("parent_session".to_string())
    );
    assert_eq!(children[0].spawned_by.as_ref().unwrap().turn_index, 1);
    assert_eq!(children[0].spawned_by.as_ref().unwrap().step_index, 2);

    // Verify top_level_only filter works
    let top_level = db
        .list_sessions(None, None, SessionOrder::default(), None, true)
        .unwrap();
    assert_eq!(top_level.len(), 1);
    assert_eq!(top_level[0].id, "parent_session");

    let all_sessions = db
        .list_sessions(None, None, SessionOrder::default(), None, false)
        .unwrap();
    assert_eq!(all_sessions.len(), 2);
}

#[test]
fn test_old_data_is_cleared_on_migration() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    // Create old schema with data
    create_old_schema_db(&db_path);

    // Open with Database::open - triggers migration
    let db = Database::open(&db_path).unwrap();

    // Old data should be gone (tables were dropped and recreated)
    let sessions = db
        .list_sessions(None, None, SessionOrder::default(), None, false)
        .unwrap();
    assert!(
        sessions.is_empty(),
        "Old sessions should be cleared after migration"
    );

    let projects = db.list_projects().unwrap();
    assert!(
        projects.is_empty(),
        "Old projects should be cleared after migration"
    );
}

#[test]
fn test_current_version_preserves_data() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    // Create DB with current schema
    let db = Database::open(&db_path).unwrap();

    let project = ProjectRecord {
        hash: ProjectHash::from("preserve_hash"),
        root_path: Some("/preserve/path".to_string()),
        last_scanned_at: None,
    };
    db.insert_or_update_project(&project).unwrap();

    let session = SessionRecord {
        id: "preserve_session".to_string(),
        project_hash: ProjectHash::from("preserve_hash"),
        repository_hash: None,
        provider: "claude_code".to_string(),
        start_ts: Some("2024-01-01T00:00:00Z".to_string()),
        end_ts: None,
        snippet: Some("preserved".to_string()),
        is_valid: true,
        parent_session_id: None,
        spawned_by: None,
    };
    db.insert_or_update_session(&session).unwrap();
    drop(db);

    // Reopen - should preserve data (no migration needed)
    let db = Database::open(&db_path).unwrap();

    let sessions = db
        .list_sessions(None, None, SessionOrder::default(), None, false)
        .unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].id, "preserve_session");
}
