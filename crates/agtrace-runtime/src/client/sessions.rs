use crate::ops::{
    ExportService, IndexProgress, IndexService, ListSessionsRequest, PackResult, PackService,
    SessionService,
};
use crate::storage::{RawFileContent, get_raw_files};
use agtrace_engine::export::ExportStrategy;
use agtrace_index::{Database, SessionSummary};
use agtrace_providers::ProviderAdapter;
use agtrace_types::AgentEvent;
use anyhow::Result;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Default)]
pub struct SessionFilter {
    pub project_hash: Option<agtrace_types::ProjectHash>,
    pub limit: usize,
    pub all_projects: bool,
    pub provider: Option<String>,
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

    pub fn project(mut self, project_hash: agtrace_types::ProjectHash) -> Self {
        self.project_hash = Some(project_hash);
        self
    }

    pub fn all_projects(mut self) -> Self {
        self.all_projects = true;
        self
    }

    pub fn provider(mut self, provider: String) -> Self {
        self.provider = Some(provider);
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
    provider_configs: Arc<Vec<(String, PathBuf)>>,
}

impl SessionOps {
    pub fn new(db: Arc<Mutex<Database>>, provider_configs: Arc<Vec<(String, PathBuf)>>) -> Self {
        Self {
            db,
            provider_configs,
        }
    }

    pub fn list(&self, filter: SessionFilter) -> Result<Vec<SessionSummary>> {
        self.ensure_index_is_fresh()?;
        self.list_without_refresh(filter)
    }

    pub fn list_without_refresh(&self, filter: SessionFilter) -> Result<Vec<SessionSummary>> {
        let db = self.db.lock().unwrap();
        let service = SessionService::new(&db);
        let request = ListSessionsRequest {
            project_hash: filter.project_hash,
            limit: filter.limit,
            all_projects: filter.all_projects,
            provider: filter.provider,
            since: filter.since,
            until: filter.until,
        };
        service.list_sessions(request)
    }

    fn ensure_index_is_fresh(&self) -> Result<()> {
        let db = self.db.lock().unwrap();

        let providers: Vec<(ProviderAdapter, PathBuf)> = self
            .provider_configs
            .iter()
            .filter_map(|(name, path)| {
                agtrace_providers::create_adapter(name)
                    .ok()
                    .map(|p| (p, path.clone()))
            })
            .collect();

        let service = IndexService::new(&db, providers);

        // Scan all projects without filtering
        let scope = agtrace_types::ProjectScope::All;

        service.run(scope, false, |_progress: IndexProgress| {})?;

        Ok(())
    }

    pub fn find(&self, session_id: &str) -> Result<SessionHandle> {
        if let Some(resolved_id) = self.resolve_session_id(session_id)? {
            return Ok(SessionHandle {
                id: resolved_id,
                db: self.db.clone(),
            });
        }

        self.ensure_index_is_fresh()?;

        if let Some(resolved_id) = self.resolve_session_id(session_id)? {
            return Ok(SessionHandle {
                id: resolved_id,
                db: self.db.clone(),
            });
        }

        anyhow::bail!("Session not found: {}", session_id)
    }

    fn resolve_session_id(&self, session_id: &str) -> Result<Option<String>> {
        let db = self.db.lock().unwrap();

        if let Some(session) = db.get_session_by_id(session_id)? {
            return Ok(Some(session.id));
        }

        db.find_session_by_prefix(session_id)
    }

    pub fn pack_context(
        &self,
        project_hash: Option<&agtrace_types::ProjectHash>,
        limit: usize,
    ) -> Result<PackResult> {
        self.ensure_index_is_fresh()?;

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
