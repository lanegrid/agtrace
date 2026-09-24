use rusqlite::{Connection, params};

use crate::{Result, records::LogFileRecord};

pub fn insert_or_update(conn: &Connection, log_file: &LogFileRecord) -> Result<()> {
    conn.execute(
        r#"
        INSERT INTO log_files (path, session_id, role, agent_id, agent_name, spawn_call_id,
                               file_size, mod_time)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        ON CONFLICT(path) DO UPDATE SET
            session_id = ?2,
            role = ?3,
            agent_id = ?4,
            agent_name = ?5,
            spawn_call_id = ?6,
            file_size = ?7,
            mod_time = ?8
        "#,
        params![
            &log_file.path,
            &log_file.session_id,
            &log_file.role,
            &log_file.agent_id,
            &log_file.agent_name,
            &log_file.spawn_call_id,
            &log_file.file_size,
            &log_file.mod_time
        ],
    )?;

    Ok(())
}

pub fn get_session_files(conn: &Connection, session_id: &str) -> Result<Vec<LogFileRecord>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT path, session_id, role, agent_id, agent_name, spawn_call_id, file_size, mod_time
        FROM log_files
        WHERE session_id = ?1
        ORDER BY CASE role WHEN 'main' THEN 0 ELSE 1 END, path
        "#,
    )?;

    let files = stmt
        .query_map([session_id], |row| {
            Ok(LogFileRecord {
                path: row.get(0)?,
                session_id: row.get(1)?,
                role: row.get(2)?,
                agent_id: row.get(3)?,
                agent_name: row.get(4)?,
                spawn_call_id: row.get(5)?,
                file_size: row.get(6)?,
                mod_time: row.get(7)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, rusqlite::Error>>()?;

    Ok(files)
}

pub fn get_all(conn: &Connection) -> Result<Vec<LogFileRecord>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT path, session_id, role, agent_id, agent_name, spawn_call_id, file_size, mod_time
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
                agent_id: row.get(3)?,
                agent_name: row.get(4)?,
                spawn_call_id: row.get(5)?,
                file_size: row.get(6)?,
                mod_time: row.get(7)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, rusqlite::Error>>()?;

    Ok(files)
}
