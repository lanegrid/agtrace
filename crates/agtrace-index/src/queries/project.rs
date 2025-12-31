use agtrace_types::ProjectHash;
use rusqlite::{Connection, OptionalExtension, params};

use crate::{Result, records::ProjectRecord};

pub fn insert_or_update(conn: &Connection, project: &ProjectRecord) -> Result<()> {
    conn.execute(
        r#"
        INSERT INTO projects (hash, root_path, last_scanned_at)
        VALUES (?1, ?2, ?3)
        ON CONFLICT(hash) DO UPDATE SET
            root_path = COALESCE(?2, root_path),
            last_scanned_at = ?3
        "#,
        params![
            project.hash.as_str(),
            &project.root_path,
            &project.last_scanned_at
        ],
    )?;

    Ok(())
}

pub fn get(conn: &Connection, hash: &str) -> Result<Option<ProjectRecord>> {
    let result = conn
        .query_row(
            r#"
        SELECT hash, root_path, last_scanned_at
        FROM projects
        WHERE hash = ?1
        "#,
            [hash],
            |row| {
                Ok(ProjectRecord {
                    hash: ProjectHash::from(row.get::<_, String>(0)?),
                    root_path: row.get(1)?,
                    last_scanned_at: row.get(2)?,
                })
            },
        )
        .optional()?;

    Ok(result)
}

pub fn list(conn: &Connection) -> Result<Vec<ProjectRecord>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT hash, root_path, last_scanned_at
        FROM projects
        ORDER BY last_scanned_at DESC
        "#,
    )?;

    let projects = stmt
        .query_map([], |row| {
            Ok(ProjectRecord {
                hash: ProjectHash::from(row.get::<_, String>(0)?),
                root_path: row.get(1)?,
                last_scanned_at: row.get(2)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, rusqlite::Error>>()?;

    Ok(projects)
}

pub fn count_sessions(conn: &Connection, project_hash: &str) -> Result<usize> {
    let count: i64 = conn.query_row(
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
