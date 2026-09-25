use crate::ops::{
    ExportService, IndexProgress, IndexService, ListSessionsRequest, PackResult, PackService,
    SessionService,
};
use crate::storage::{RawFileContent, get_raw_files};
use crate::{Error, Result};
use agtrace_engine::export::ExportStrategy;
use agtrace_engine::workspace::{
    TeammateSpawn, find_teammate_spawn, teammate_parent, teammate_spawns,
};
use agtrace_index::{Database, SessionSummary};
use agtrace_providers::ProviderAdapter;
use agtrace_types::{AgentEvent, AgentId};
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
    #[cfg(test)]
    pub(crate) fn for_tests(id: &str, db: Arc<Mutex<Database>>) -> Self {
        Self {
            id: id.to_string(),
            db,
        }
    }

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
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    /// Agent tree of this session (design §6.4): the session's own agent, its Claude
    /// subagents / forks (log files of the session), and child sessions (Codex child
    /// threads and forks, Claude teammates) recursively. A teammate is placed with
    /// the live view's rule ([`teammate_parent`]): under the agent of this session
    /// whose log spawned it, else under the team lead (this session).
    pub fn agent_tree(&self) -> Result<AgentNode> {
        let db = self.db.lock().unwrap();
        let summary = db
            .get_session_by_id(&self.id)?
            .ok_or_else(|| Error::InvalidOperation(format!("Session not found: {}", self.id)))?;
        let mut visited = std::collections::HashSet::new();
        session_node(&db, summary, &mut visited)
    }
}

/// One agent in a session's agent tree.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AgentNode {
    /// Agent id (`claude:<sid>`, `claude:<sid>/<aid>`, `codex:<thread>`).
    pub agent_id: String,
    /// Index session the agent's log belongs to.
    pub session_id: String,
    pub provider: String,
    /// `main`, `subagent`, `fork`, `teammate` or `codex_thread`.
    pub kind: String,
    /// Display name (teammate name, subagent description, Codex path leaf).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Codex `agent_path` (`/root/judge`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Provider call id of the spawning tool call in the parent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spawn_call_id: Option<String>,
    /// First user prompt (session owners only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_ts: Option<String>,
    /// Log file of the agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_file: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<AgentNode>,
}

impl AgentNode {
    /// Pre-order walk of the tree (self first).
    pub fn walk(&self) -> Vec<&AgentNode> {
        let mut out = vec![self];
        for child in &self.children {
            out.extend(child.walk());
        }
        out
    }
}

fn session_node(
    db: &Database,
    summary: SessionSummary,
    visited: &mut std::collections::HashSet<String>,
) -> Result<AgentNode> {
    visited.insert(summary.id.clone());
    let files = db.get_session_files(&summary.id)?;
    let main = files.iter().find(|f| f.role == "main");
    let mut node = AgentNode {
        agent_id: main
            .map(|f| f.agent_id.clone())
            .unwrap_or_else(|| summary.id.clone()),
        session_id: summary.id.clone(),
        provider: summary.provider.clone(),
        kind: summary.agent_kind.clone(),
        name: summary.agent_name.clone(),
        path: summary.agent_path.clone(),
        spawn_call_id: summary.spawn_call_id.clone(),
        snippet: summary.snippet.clone(),
        start_ts: summary.start_ts.clone(),
        log_file: main.map(|f| f.path.clone()),
        children: Vec::new(),
    };
    // Claude subagents / forks: log files of this session.
    for file in files.iter().filter(|f| f.role != "main") {
        node.children.push(AgentNode {
            agent_id: file.agent_id.clone(),
            session_id: summary.id.clone(),
            provider: summary.provider.clone(),
            kind: file.role.clone(),
            name: file.agent_name.clone(),
            path: None,
            spawn_call_id: file.spawn_call_id.clone(),
            snippet: None,
            start_ts: None,
            log_file: Some(file.path.clone()),
            children: Vec::new(),
        });
    }
    // Child sessions (Codex threads / forks, teammates), recursively.
    let children = db.get_child_sessions(&summary.id)?;
    let spawns = if children.iter().any(|c| c.agent_kind == "teammate") {
        teammate_spawns_in(&files)
    } else {
        Vec::new()
    };
    for child in children {
        if visited.contains(&child.id) {
            continue;
        }
        let teammate = (child.agent_kind == "teammate").then(|| {
            let started = child
                .start_ts
                .as_deref()
                .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
                .map(|t| t.with_timezone(&chrono::Utc));
            find_teammate_spawn(
                &spawns,
                child.team_name.as_deref(),
                child.agent_name.as_deref().unwrap_or_default(),
                started,
            )
            .cloned()
        });
        let mut child_node = session_node(db, child, visited)?;
        let Some(spawn) = teammate else {
            node.children.push(child_node);
            continue;
        };
        // Shared rule with the live view: the spawning agent, else the team lead
        // (this session, whose team config links the teammate here).
        if child_node.spawn_call_id.is_none() {
            child_node.spawn_call_id = spawn.as_ref().and_then(|s| s.spawn_call_id.clone());
        }
        let lead = AgentId::parse(&node.agent_id);
        let me = AgentId::parse(&child_node.agent_id);
        let parent = me
            .and_then(|me| teammate_parent(&me, spawn.as_ref().map(|s| &s.spawner), lead.as_ref()));
        let target = parent
            .and_then(|p| node.children.iter_mut().find(|c| c.agent_id == p.as_str()))
            .filter(|c| c.session_id == summary.id);
        match target {
            Some(spawner) => spawner.children.push(child_node),
            None => node.children.push(child_node),
        }
    }
    Ok(node)
}

/// Teammate spawns in a Claude session's own log files (main + subagents / forks).
fn teammate_spawns_in(files: &[agtrace_index::LogFileRecord]) -> Vec<TeammateSpawn> {
    files
        .iter()
        .filter(|f| f.agent_id.starts_with("claude:"))
        .filter_map(|f| {
            agtrace_providers::normalize_claude_file(std::path::Path::new(&f.path)).ok()
        })
        .flat_map(|events| teammate_spawns(&events))
        .collect()
}
