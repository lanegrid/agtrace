use agtrace_types::{ProjectHash, RepositoryHash, SessionOrder, SpawnContext};
use rusqlite::{Connection, params};

use crate::{
    Error, Result,
    records::{SessionRecord, SessionSummary},
};

pub fn insert_or_update(conn: &Connection, session: &SessionRecord) -> Result<()> {
    let (spawned_by_turn, spawned_by_step) = match &session.spawned_by {
        Some(ctx) => (Some(ctx.turn_index as i64), Some(ctx.step_index as i64)),
        None => (None, None),
    };

    let repository_hash_str = session.repository_hash.as_ref().map(|h| h.as_str());

    conn.execute(
        r#"
        INSERT INTO sessions (id, project_hash, repository_hash, provider, start_ts, end_ts, snippet, is_valid,
                              parent_session_id, spawned_by_turn, spawned_by_step)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
        ON CONFLICT(id) DO UPDATE SET
            project_hash = ?2,
            repository_hash = COALESCE(?3, repository_hash),
            provider = ?4,
            start_ts = COALESCE(?5, start_ts),
            end_ts = COALESCE(?6, end_ts),
            snippet = COALESCE(?7, snippet),
            is_valid = ?8,
            parent_session_id = COALESCE(?9, parent_session_id),
            spawned_by_turn = COALESCE(?10, spawned_by_turn),
            spawned_by_step = COALESCE(?11, spawned_by_step)
        "#,
        params![
            &session.id,
            session.project_hash.as_str(),
            repository_hash_str,
            &session.provider,
            &session.start_ts,
            &session.end_ts,
            &session.snippet,
            &session.is_valid,
            &session.parent_session_id,
            spawned_by_turn,
            spawned_by_step
        ],
    )?;

    Ok(())
}

pub fn get_by_id(conn: &Connection, session_id: &str) -> Result<Option<SessionSummary>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT s.id, s.provider, s.project_hash, s.repository_hash, p.root_path, s.start_ts, s.snippet,
               s.parent_session_id, s.spawned_by_turn, s.spawned_by_step
        FROM sessions s
        LEFT JOIN projects p ON s.project_hash = p.hash
        WHERE s.id = ?1 AND s.is_valid = 1
        "#,
    )?;

    let mut rows = stmt.query([session_id])?;
    if let Some(row) = rows.next()? {
        let spawned_by = match (row.get::<_, Option<i64>>(8)?, row.get::<_, Option<i64>>(9)?) {
            (Some(turn), Some(step)) => Some(SpawnContext {
                turn_index: turn as usize,
                step_index: step as usize,
            }),
            _ => None,
        };

        Ok(Some(SessionSummary {
            id: row.get(0)?,
            provider: row.get(1)?,
            project_hash: ProjectHash::from(row.get::<_, String>(2)?),
            repository_hash: row.get::<_, Option<String>>(3)?.map(RepositoryHash::from),
            project_root: row.get(4)?,
            start_ts: row.get(5)?,
            snippet: row.get(6)?,
            parent_session_id: row.get(7)?,
            spawned_by,
        }))
    } else {
        Ok(None)
    }
}

pub fn list(
    conn: &Connection,
    project_hash: Option<&ProjectHash>,
    provider: Option<&str>,
    order: SessionOrder,
    limit: Option<usize>,
    top_level_only: bool,
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

    if top_level_only {
        where_clauses.push("parent_session_id IS NULL");
    }

    let where_clause = where_clauses.join(" AND ");
    let order_clause = match order {
        SessionOrder::NewestFirst => "ORDER BY s.end_ts DESC, s.start_ts DESC",
        SessionOrder::OldestFirst => "ORDER BY s.start_ts ASC, s.end_ts ASC",
    };
    let limit_clause = limit.map(|l| format!("LIMIT {}", l)).unwrap_or_default();

    let query = format!(
        r#"
        SELECT s.id, s.provider, s.project_hash, s.repository_hash, p.root_path, s.start_ts, s.snippet,
               s.parent_session_id, s.spawned_by_turn, s.spawned_by_step
        FROM sessions s
        LEFT JOIN projects p ON s.project_hash = p.hash
        WHERE {}
        {}
        {}
        "#,
        where_clause, order_clause, limit_clause
    );

    let mut stmt = conn.prepare(&query)?;
    let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let sessions = stmt
        .query_map(param_refs.as_slice(), |row| {
            let spawned_by = match (row.get::<_, Option<i64>>(8)?, row.get::<_, Option<i64>>(9)?) {
                (Some(turn), Some(step)) => Some(SpawnContext {
                    turn_index: turn as usize,
                    step_index: step as usize,
                }),
                _ => None,
            };

            Ok(SessionSummary {
                id: row.get(0)?,
                provider: row.get(1)?,
                project_hash: ProjectHash::from(row.get::<_, String>(2)?),
                repository_hash: row.get::<_, Option<String>>(3)?.map(RepositoryHash::from),
                project_root: row.get(4)?,
                start_ts: row.get(5)?,
                snippet: row.get(6)?,
                parent_session_id: row.get(7)?,
                spawned_by,
            })
        })?
        .collect::<std::result::Result<Vec<_>, rusqlite::Error>>()?;

    Ok(sessions)
}

/// Get child sessions (subagents) that were spawned from a parent session
pub fn get_children(conn: &Connection, parent_session_id: &str) -> Result<Vec<SessionSummary>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT s.id, s.provider, s.project_hash, s.repository_hash, p.root_path, s.start_ts, s.snippet,
               s.parent_session_id, s.spawned_by_turn, s.spawned_by_step
        FROM sessions s
        LEFT JOIN projects p ON s.project_hash = p.hash
        WHERE s.parent_session_id = ?1 AND s.is_valid = 1
        ORDER BY s.spawned_by_turn ASC, s.spawned_by_step ASC
        "#,
    )?;

    let sessions = stmt
        .query_map([parent_session_id], |row| {
            let spawned_by = match (row.get::<_, Option<i64>>(8)?, row.get::<_, Option<i64>>(9)?) {
                (Some(turn), Some(step)) => Some(SpawnContext {
                    turn_index: turn as usize,
                    step_index: step as usize,
                }),
                _ => None,
            };

            Ok(SessionSummary {
                id: row.get(0)?,
                provider: row.get(1)?,
                project_hash: ProjectHash::from(row.get::<_, String>(2)?),
                repository_hash: row.get::<_, Option<String>>(3)?.map(RepositoryHash::from),
                project_root: row.get(4)?,
                start_ts: row.get(5)?,
                snippet: row.get(6)?,
                parent_session_id: row.get(7)?,
                spawned_by,
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
            repository_hash: None,
            provider: "claude_code".to_string(),
            start_ts: Some("2024-01-01T00:00:00Z".to_string()),
            end_ts: None,
            snippet: Some("test1".to_string()),
            is_valid: true,
            parent_session_id: None,
            spawned_by: None,
        };

        let session2 = SessionRecord {
            id: "session2".to_string(),
            project_hash: project_hash.clone(),
            repository_hash: None,
            provider: "codex".to_string(),
            start_ts: Some("2024-01-02T00:00:00Z".to_string()),
            end_ts: None,
            snippet: Some("test2".to_string()),
            is_valid: true,
            parent_session_id: None,
            spawned_by: None,
        };

        let session3 = SessionRecord {
            id: "session3".to_string(),
            project_hash: project_hash.clone(),
            repository_hash: None,
            provider: "gemini".to_string(),
            start_ts: Some("2024-01-03T00:00:00Z".to_string()),
            end_ts: None,
            snippet: Some("test3".to_string()),
            is_valid: true,
            parent_session_id: None,
            spawned_by: None,
        };

        db.insert_or_update_session(&session1)?;
        db.insert_or_update_session(&session2)?;
        db.insert_or_update_session(&session3)?;

        // Test filter by provider
        let sessions =
            db.list_sessions(None, Some("codex"), SessionOrder::default(), None, false)?;
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].provider, "codex");

        let sessions =
            db.list_sessions(None, Some("gemini"), SessionOrder::default(), None, false)?;
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].provider, "gemini");

        let sessions = db.list_sessions(
            None,
            Some("claude_code"),
            SessionOrder::default(),
            None,
            false,
        )?;
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].provider, "claude_code");

        // Test no filter returns all
        let sessions = db.list_sessions(None, None, SessionOrder::default(), None, false)?;
        assert_eq!(sessions.len(), 3);

        // Test combined project_hash and provider filter
        let sessions = db.list_sessions(
            Some(&project_hash),
            Some("codex"),
            SessionOrder::default(),
            None,
            false,
        )?;
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].provider, "codex");

        Ok(())
    }

    #[test]
    fn test_session_ordering() -> Result<()> {
        let db = Database::open_in_memory()?;
        let project_hash = ProjectHash::from("test_project".to_string());

        // Insert project first to satisfy foreign key constraint
        let project = crate::records::ProjectRecord {
            hash: project_hash.clone(),
            root_path: Some("/test/path".to_string()),
            last_scanned_at: None,
        };
        db.insert_or_update_project(&project)?;

        // Insert sessions with different timestamps
        let session1 = SessionRecord {
            id: "session1".to_string(),
            project_hash: project_hash.clone(),
            repository_hash: None,
            provider: "claude_code".to_string(),
            start_ts: Some("2024-01-01T00:00:00Z".to_string()),
            end_ts: Some("2024-01-01T01:00:00Z".to_string()),
            snippet: Some("First session".to_string()),
            is_valid: true,
            parent_session_id: None,
            spawned_by: None,
        };

        let session2 = SessionRecord {
            id: "session2".to_string(),
            project_hash: project_hash.clone(),
            repository_hash: None,
            provider: "claude_code".to_string(),
            start_ts: Some("2024-01-02T00:00:00Z".to_string()),
            end_ts: Some("2024-01-02T01:00:00Z".to_string()),
            snippet: Some("Second session".to_string()),
            is_valid: true,
            parent_session_id: None,
            spawned_by: None,
        };

        let session3 = SessionRecord {
            id: "session3".to_string(),
            project_hash: project_hash.clone(),
            repository_hash: None,
            provider: "claude_code".to_string(),
            start_ts: Some("2024-01-03T00:00:00Z".to_string()),
            end_ts: Some("2024-01-03T01:00:00Z".to_string()),
            snippet: Some("Third session".to_string()),
            is_valid: true,
            parent_session_id: None,
            spawned_by: None,
        };

        db.insert_or_update_session(&session1)?;
        db.insert_or_update_session(&session2)?;
        db.insert_or_update_session(&session3)?;

        // Test NewestFirst ordering (default)
        let sessions = db.list_sessions(
            Some(&project_hash),
            None,
            SessionOrder::NewestFirst,
            None,
            false,
        )?;
        assert_eq!(sessions.len(), 3);
        assert_eq!(sessions[0].id, "session3");
        assert_eq!(sessions[1].id, "session2");
        assert_eq!(sessions[2].id, "session1");

        // Test OldestFirst ordering
        let sessions = db.list_sessions(
            Some(&project_hash),
            None,
            SessionOrder::OldestFirst,
            None,
            false,
        )?;
        assert_eq!(sessions.len(), 3);
        assert_eq!(sessions[0].id, "session1");
        assert_eq!(sessions[1].id, "session2");
        assert_eq!(sessions[2].id, "session3");

        Ok(())
    }
}
