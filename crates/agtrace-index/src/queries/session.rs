use agtrace_types::ProjectHash;
use rusqlite::{Connection, params};

use crate::{records::{SessionRecord, SessionSummary}, Error, Result};

pub fn insert_or_update(conn: &Connection, session: &SessionRecord) -> Result<()> {
    conn.execute(
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
            session.project_hash.as_str(),
            &session.provider,
            &session.start_ts,
            &session.end_ts,
            &session.snippet,
            &session.is_valid
        ],
    )?;

    Ok(())
}

pub fn get_by_id(conn: &Connection, session_id: &str) -> Result<Option<SessionSummary>> {
    let mut stmt = conn.prepare(
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
            project_hash: ProjectHash::from(row.get::<_, String>(2)?),
            start_ts: row.get(3)?,
            snippet: row.get(4)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn list(
    conn: &Connection,
    project_hash: Option<&ProjectHash>,
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
            hash.as_str(),
            limit
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

    let mut stmt = conn.prepare(&query)?;
    let sessions = stmt
        .query_map([], |row| {
            Ok(SessionSummary {
                id: row.get(0)?,
                provider: row.get(1)?,
                project_hash: ProjectHash::from(row.get::<_, String>(2)?),
                start_ts: row.get(3)?,
                snippet: row.get(4)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, rusqlite::Error>>()?;

    Ok(sessions)
}

pub fn find_by_prefix(conn: &Connection, prefix: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare(
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
        .collect::<std::result::Result<Vec<_>, rusqlite::Error>>()?;

    match matches.len() {
        0 => Ok(None),
        1 => Ok(Some(matches.remove(0))),
        _ => Err(Error::Query(format!(
            "Ambiguous session ID prefix '{}': multiple sessions match",
            prefix
        ))),
    }
}
