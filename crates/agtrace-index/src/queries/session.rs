use agtrace_types::ProjectHash;
use rusqlite::{Connection, params};

use crate::{
    Error, Result,
    records::{SessionRecord, SessionSummary},
};

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
        SELECT s.id, s.provider, s.project_hash, p.root_path, s.start_ts, s.snippet
        FROM sessions s
        LEFT JOIN projects p ON s.project_hash = p.hash
        WHERE s.id = ?1 AND s.is_valid = 1
        "#,
    )?;

    let mut rows = stmt.query([session_id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(SessionSummary {
            id: row.get(0)?,
            provider: row.get(1)?,
            project_hash: ProjectHash::from(row.get::<_, String>(2)?),
            project_root: row.get(3)?,
            start_ts: row.get(4)?,
            snippet: row.get(5)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn list(
    conn: &Connection,
    project_hash: Option<&ProjectHash>,
    provider: Option<&str>,
    limit: Option<usize>,
) -> Result<Vec<SessionSummary>> {
    let mut where_clauses = vec!["is_valid = 1"];
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(hash) = project_hash {
        where_clauses.push("project_hash = ?");
        params.push(Box::new(hash.as_str().to_string()));
    }

    if let Some(prov) = provider {
        where_clauses.push("provider = ?");
        params.push(Box::new(prov.to_string()));
    }

    let where_clause = where_clauses.join(" AND ");
    let limit_clause = limit.map(|l| format!("LIMIT {}", l)).unwrap_or_default();

    let query = format!(
        r#"
        SELECT s.id, s.provider, s.project_hash, p.root_path, s.start_ts, s.snippet
        FROM sessions s
        LEFT JOIN projects p ON s.project_hash = p.hash
        WHERE {}
        ORDER BY s.start_ts DESC
        {}
        "#,
        where_clause, limit_clause
    );

    let mut stmt = conn.prepare(&query)?;
    let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let sessions = stmt
        .query_map(param_refs.as_slice(), |row| {
            Ok(SessionSummary {
                id: row.get(0)?,
                provider: row.get(1)?,
                project_hash: ProjectHash::from(row.get::<_, String>(2)?),
                project_root: row.get(3)?,
                start_ts: row.get(4)?,
                snippet: row.get(5)?,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;
    use crate::records::SessionRecord;

    #[test]
    fn test_list_with_provider_filter() -> Result<()> {
        let db = Database::open_in_memory()?;
        let project_hash = ProjectHash::from("test_project".to_string());

        // Insert project first to satisfy foreign key constraint
        let project = crate::records::ProjectRecord {
            hash: project_hash.clone(),
            root_path: Some("/test/path".to_string()),
            last_scanned_at: None,
        };
        db.insert_or_update_project(&project)?;

        let session1 = SessionRecord {
            id: "session1".to_string(),
            project_hash: project_hash.clone(),
            provider: "claude_code".to_string(),
            start_ts: Some("2024-01-01T00:00:00Z".to_string()),
            end_ts: None,
            snippet: Some("test1".to_string()),
            is_valid: true,
        };

        let session2 = SessionRecord {
            id: "session2".to_string(),
            project_hash: project_hash.clone(),
            provider: "codex".to_string(),
            start_ts: Some("2024-01-02T00:00:00Z".to_string()),
            end_ts: None,
            snippet: Some("test2".to_string()),
            is_valid: true,
        };

        let session3 = SessionRecord {
            id: "session3".to_string(),
            project_hash: project_hash.clone(),
            provider: "gemini".to_string(),
            start_ts: Some("2024-01-03T00:00:00Z".to_string()),
            end_ts: None,
            snippet: Some("test3".to_string()),
            is_valid: true,
        };

        db.insert_or_update_session(&session1)?;
        db.insert_or_update_session(&session2)?;
        db.insert_or_update_session(&session3)?;

        // Test filter by provider
        let sessions = db.list_sessions(None, Some("codex"), None)?;
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].provider, "codex");

        let sessions = db.list_sessions(None, Some("gemini"), None)?;
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].provider, "gemini");

        let sessions = db.list_sessions(None, Some("claude_code"), None)?;
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].provider, "claude_code");

        // Test no filter returns all
        let sessions = db.list_sessions(None, None, None)?;
        assert_eq!(sessions.len(), 3);

        // Test combined project_hash and provider filter
        let sessions = db.list_sessions(Some(&project_hash), Some("codex"), None)?;
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].provider, "codex");

        Ok(())
    }
}
