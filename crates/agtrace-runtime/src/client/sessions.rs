use crate::ops::{ExportService, ListSessionsRequest, PackResult, PackService, SessionService};
use crate::storage::{get_raw_files, RawFileContent};
use agtrace_engine::export::ExportStrategy;
use agtrace_index::{Database, SessionSummary};
use agtrace_types::AgentEvent;
use anyhow::Result;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Default)]
pub struct SessionFilter {
    pub project_hash: Option<String>,
    pub limit: usize,
    pub all_projects: bool,
    pub source: Option<String>,
    pub since: Option<String>,
    pub until: Option<String>,
}

impl SessionFilter {
    pub fn new() -> Self {
        Self {
            limit: 100,
            ..Default::default()
        }
    }

    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    pub fn project(mut self, project_hash: String) -> Self {
        self.project_hash = Some(project_hash);
        self
    }

    pub fn all_projects(mut self) -> Self {
        self.all_projects = true;
        self
    }

    pub fn source(mut self, source: String) -> Self {
        self.source = Some(source);
        self
    }

    pub fn since(mut self, since: String) -> Self {
        self.since = Some(since);
        self
    }

    pub fn until(mut self, until: String) -> Self {
        self.until = Some(until);
        self
    }
}

pub struct SessionOps {
    db: Arc<Mutex<Database>>,
}

impl SessionOps {
    pub fn new(db: Arc<Mutex<Database>>) -> Self {
        Self { db }
    }

    pub fn list(&self, filter: SessionFilter) -> Result<Vec<SessionSummary>> {
        let db = self.db.lock().unwrap();
        let service = SessionService::new(&db);
        let request = ListSessionsRequest {
            project_hash: filter.project_hash,
            limit: filter.limit,
            all_projects: filter.all_projects,
            source: filter.source,
            since: filter.since,
            until: filter.until,
        };
        service.list_sessions(request)
    }

    pub fn find(&self, session_id: &str) -> Result<SessionHandle> {
        Ok(SessionHandle {
            id: session_id.to_string(),
            db: self.db.clone(),
        })
    }

    pub fn pack_context(&self, project_hash: Option<&str>, limit: usize) -> Result<PackResult> {
        let db = self.db.lock().unwrap();
        let service = PackService::new(&db);
        service.select_sessions(project_hash, limit)
    }
}

pub struct SessionHandle {
    id: String,
    db: Arc<Mutex<Database>>,
}

impl SessionHandle {
    pub fn events(&self) -> Result<Vec<AgentEvent>> {
        let db = self.db.lock().unwrap();
        let service = SessionService::new(&db);
        service.get_session_events(&self.id)
    }

    pub fn raw_files(&self) -> Result<Vec<RawFileContent>> {
        let db = self.db.lock().unwrap();
        get_raw_files(&db, &self.id)
    }

    pub fn export(&self, strategy: ExportStrategy) -> Result<Vec<AgentEvent>> {
        let db = self.db.lock().unwrap();
        let service = ExportService::new(&db);
        service.export_session(&self.id, strategy)
    }

    pub fn id(&self) -> &str {
        &self.id
    }
}
