use crate::ops::{
    ExportService, IndexProgress, IndexService, ListSessionsRequest, PackResult, PackService,
    SessionService,
};
use crate::storage::{RawFileContent, get_raw_files};
use crate::{Error, Result};
use agtrace_engine::export::ExportStrategy;
use agtrace_index::{Database, SessionSummary};
use agtrace_providers::ProviderAdapter;
use agtrace_types::AgentEvent;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct SessionFilter {
    pub scope: agtrace_types::ProjectScope,
    pub limit: Option<usize>,
    pub provider: Option<String>,
    pub order: agtrace_types::SessionOrder,
    pub since: Option<String>,
    pub until: Option<String>,
    pub top_level_only: bool,
}

impl SessionFilter {
    /// Create a filter for all projects (top-level sessions only by default)
    pub fn all() -> Self {
        Self {
            scope: agtrace_types::ProjectScope::All,
            limit: None,
            provider: None,
            order: agtrace_types::SessionOrder::default(),
            since: None,
            until: None,
            top_level_only: true,
        }
    }

    /// Create a filter for a specific project (top-level sessions only by default)
    pub fn project(project_hash: agtrace_types::ProjectHash) -> Self {
        Self {
            scope: agtrace_types::ProjectScope::Specific(project_hash),
            limit: None,
            provider: None,
            order: agtrace_types::SessionOrder::default(),
            since: None,
            until: None,
            top_level_only: true,
        }
    }

    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn provider(mut self, provider: String) -> Self {
        self.provider = Some(provider);
        self
    }

    pub fn order(mut self, order: agtrace_types::SessionOrder) -> Self {
        self.order = order;
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

    /// Include child sessions (subagents) in the results
    pub fn include_children(mut self) -> Self {
        self.top_level_only = false;
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
            scope: filter.scope,
            limit: filter.limit,
            provider: filter.provider,
            order: filter.order,
            since: filter.since,
            until: filter.until,
            top_level_only: filter.top_level_only,
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

        Err(Error::InvalidOperation(format!(
            "Session not found: {}",
            session_id
        )))
    }

    fn resolve_session_id(&self, session_id: &str) -> Result<Option<String>> {
        let db = self.db.lock().unwrap();

        if let Some(session) = db.get_session_by_id(session_id)? {
            return Ok(Some(session.id));
        }

        Ok(db.find_session_by_prefix(session_id)?)
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

    pub fn metadata(&self) -> Result<agtrace_types::SessionMetadata> {
        let db = self.db.lock().unwrap();
        let index_summary = db.get_session_by_id(&self.id)?.ok_or_else(|| {
            Error::InvalidOperation(format!("Session metadata not found: {}", self.id))
        })?;

        // Resolve project_root from project_hash
        let project_root = db
            .get_project(index_summary.project_hash.as_str())?
            .and_then(|p| p.root_path);

        Ok(agtrace_types::SessionMetadata {
            session_id: index_summary.id.clone(),
            project_hash: index_summary.project_hash,
            project_root,
            provider: index_summary.provider,
            parent_session_id: index_summary.parent_session_id,
            spawned_by: index_summary.spawned_by,
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get child sessions (subagents) that were spawned from this session.
    pub fn child_sessions(&self) -> Result<Vec<agtrace_index::SessionSummary>> {
        let db = self.db.lock().unwrap();
        db.get_child_sessions(&self.id)
            .map_err(|e| Error::InvalidOperation(format!("Failed to get child sessions: {}", e)))
    }
}
