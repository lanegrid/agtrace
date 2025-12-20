use crate::session_repository::{LoadOptions, SessionRepository};
use agtrace_index::{Database, SessionSummary};
use agtrace_types::{resolve_effective_project_hash, AgentEvent, EventPayload};
use anyhow::Result;
use chrono::DateTime;

pub struct ListSessionsRequest {
    pub project_hash: Option<String>,
    pub limit: usize,
    pub all_projects: bool,
    pub source: Option<String>,
    pub since: Option<String>,
    pub until: Option<String>,
}

pub struct EventFilters {
    pub hide: Option<Vec<String>>,
    pub only: Option<Vec<String>>,
}

pub struct RawFileContent {
    pub path: String,
    pub content: String,
}

pub struct SessionService<'a> {
    db: &'a Database,
}

impl<'a> SessionService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn list_sessions(&self, request: ListSessionsRequest) -> Result<Vec<SessionSummary>> {
        let (effective_hash_string, _all_projects) =
            resolve_effective_project_hash(request.project_hash.as_deref(), request.all_projects)?;
        let effective_project_hash = effective_hash_string.as_deref();

        let fetch_limit = request.limit * 3;
        let mut sessions = self.db.list_sessions(effective_project_hash, fetch_limit)?;

        if let Some(src) = request.source {
            sessions.retain(|s| s.provider == src);
        }

        if let Some(since_str) = request.since {
            if let Ok(since_dt) = DateTime::parse_from_rfc3339(&since_str) {
                sessions.retain(|s| {
                    if let Some(ts) = &s.start_ts {
                        if let Ok(dt) = DateTime::parse_from_rfc3339(ts) {
                            return dt >= since_dt;
                        }
                    }
                    false
                });
            }
        }

        if let Some(until_str) = request.until {
            if let Ok(until_dt) = DateTime::parse_from_rfc3339(&until_str) {
                sessions.retain(|s| {
                    if let Some(ts) = &s.start_ts {
                        if let Ok(dt) = DateTime::parse_from_rfc3339(ts) {
                            return dt <= until_dt;
                        }
                    }
                    false
                });
            }
        }

        sessions.truncate(request.limit);

        Ok(sessions)
    }

    pub fn filter_events(&self, events: &[AgentEvent], filters: EventFilters) -> Vec<AgentEvent> {
        let mut filtered = events.to_vec();

        if let Some(only_patterns) = filters.only {
            filtered.retain(|e| {
                only_patterns.iter().any(|pattern| {
                    let pattern_lower = pattern.to_lowercase();
                    match &e.payload {
                        EventPayload::User(_) => pattern_lower == "user",
                        EventPayload::Message(_) => {
                            pattern_lower == "assistant" || pattern_lower == "message"
                        }
                        EventPayload::ToolCall(_) | EventPayload::ToolResult(_) => {
                            pattern_lower == "tool"
                        }
                        EventPayload::Reasoning(_) => pattern_lower == "reasoning",
                        EventPayload::TokenUsage(_) => {
                            pattern_lower == "token" || pattern_lower == "tokenusage"
                        }
                        EventPayload::Notification(_) => {
                            pattern_lower == "notification" || pattern_lower == "info"
                        }
                    }
                })
            });
        }

        if let Some(hide_patterns) = filters.hide {
            filtered.retain(|e| {
                !hide_patterns.iter().any(|pattern| {
                    let pattern_lower = pattern.to_lowercase();
                    match &e.payload {
                        EventPayload::User(_) => pattern_lower == "user",
                        EventPayload::Message(_) => {
                            pattern_lower == "assistant" || pattern_lower == "message"
                        }
                        EventPayload::ToolCall(_) | EventPayload::ToolResult(_) => {
                            pattern_lower == "tool"
                        }
                        EventPayload::Reasoning(_) => pattern_lower == "reasoning",
                        EventPayload::TokenUsage(_) => {
                            pattern_lower == "token" || pattern_lower == "tokenusage"
                        }
                        EventPayload::Notification(_) => {
                            pattern_lower == "notification" || pattern_lower == "info"
                        }
                    }
                })
            });
        }

        filtered
    }

    pub fn get_session_events(&self, session_id: &str) -> Result<Vec<AgentEvent>> {
        let loader = SessionRepository::new(self.db);
        let options = LoadOptions::default();
        loader.load_events(session_id, &options)
    }

    pub fn get_raw_files(&self, session_id: &str) -> Result<Vec<RawFileContent>> {
        let log_files = self.db.get_session_files(session_id)?;

        let mut contents = Vec::new();
        for log_file in &log_files {
            let content = std::fs::read_to_string(&log_file.path)?;
            contents.push(RawFileContent {
                path: log_file.path.clone(),
                content,
            });
        }

        Ok(contents)
    }
}
