use anyhow::Result;
use rusqlite::{Connection, params};

use crate::records::LogFileRecord;

pub fn insert_or_update(conn: &Connection, log_file: &LogFileRecord) -> Result<()> {
    conn.execute(
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

pub fn get_session_files(conn: &Connection, session_id: &str) -> Result<Vec<LogFileRecord>> {
    let mut stmt = conn.prepare(
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

pub fn get_all(conn: &Connection) -> Result<Vec<LogFileRecord>> {
    let mut stmt = conn.prepare(
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
