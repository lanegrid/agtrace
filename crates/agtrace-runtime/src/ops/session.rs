use crate::Result;
use crate::storage::{LoadOptions, SessionRepository};
use agtrace_index::{Database, SessionSummary};
use agtrace_types::AgentEvent;
use chrono::DateTime;

pub struct ListSessionsRequest {
    pub scope: agtrace_types::ProjectScope,
    pub limit: Option<usize>,
    pub provider: Option<String>,
    pub order: agtrace_types::SessionOrder,
    pub since: Option<String>,
    pub until: Option<String>,
    pub top_level_only: bool,
}

pub struct SessionService<'a> {
    db: &'a Database,
}

impl<'a> SessionService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn list_sessions(&self, request: ListSessionsRequest) -> Result<Vec<SessionSummary>> {
        let effective_project_hash = match &request.scope {
            agtrace_types::ProjectScope::All => None,
            agtrace_types::ProjectScope::Specific(hash) => Some(hash.clone()),
        };

        let mut sessions = self.db.list_sessions(
            effective_project_hash.as_ref(),
            request.provider.as_deref(),
            request.order,
            request.limit,
            request.top_level_only,
        )?;

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

        if let Some(limit) = request.limit {
            sessions.truncate(limit);
        }

        Ok(sessions)
    }

    pub fn get_session_events(&self, session_id: &str) -> Result<Vec<AgentEvent>> {
        let loader = SessionRepository::new(self.db);
        let options = LoadOptions::default();
        loader.load_events(session_id, &options)
    }
}
