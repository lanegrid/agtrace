use crate::storage::{LoadOptions, SessionRepository};
use agtrace_index::{Database, SessionSummary};
use agtrace_types::{AgentEvent, resolve_effective_project_hash};
use anyhow::Result;
use chrono::DateTime;

pub struct ListSessionsRequest {
    pub project_hash: Option<agtrace_types::ProjectHash>,
    pub limit: usize,
    pub all_projects: bool,
    pub provider: Option<String>,
    pub since: Option<String>,
    pub until: Option<String>,
}

pub struct SessionService<'a> {
    db: &'a Database,
}

impl<'a> SessionService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn list_sessions(&self, request: ListSessionsRequest) -> Result<Vec<SessionSummary>> {
        let (effective_hash_string, _all_projects) = resolve_effective_project_hash(
            request.project_hash.as_ref().map(|h| h.as_str()),
            request.all_projects,
        )?;
        let effective_project_hash = effective_hash_string.as_deref();

        let fetch_limit = request.limit * 3;
        let mut sessions = self.db.list_sessions(effective_project_hash, fetch_limit)?;

        if let Some(src) = request.provider {
            sessions.retain(|s| s.provider == src);
        }

        if let Some(since_str) = request.since
            && let Ok(since_dt) = DateTime::parse_from_rfc3339(&since_str)
        {
            sessions.retain(|s| {
                if let Some(ts) = &s.start_ts
                    && let Ok(dt) = DateTime::parse_from_rfc3339(ts)
                {
                    return dt >= since_dt;
                }
                false
            });
        }

        if let Some(until_str) = request.until
            && let Ok(until_dt) = DateTime::parse_from_rfc3339(&until_str)
        {
            sessions.retain(|s| {
                if let Some(ts) = &s.start_ts
                    && let Ok(dt) = DateTime::parse_from_rfc3339(ts)
                {
                    return dt <= until_dt;
                }
                false
            });
        }

        sessions.truncate(request.limit);

        Ok(sessions)
    }

    pub fn get_session_events(&self, session_id: &str) -> Result<Vec<AgentEvent>> {
        let loader = SessionRepository::new(self.db);
        let options = LoadOptions::default();
        loader.load_events(session_id, &options)
    }
}
