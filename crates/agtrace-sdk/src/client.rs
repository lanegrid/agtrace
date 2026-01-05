use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::error::{Error, Result};
use crate::query::{
    Cursor, EventMatch, GetTurnsArgs, GetTurnsResponse, ListTurnsArgs, ListTurnsResponse,
    SearchEventsArgs, SearchEventsResponse,
};
use crate::types::*;
use crate::watch::WatchBuilder;

// ============================================================================
// ClientBuilder
// ============================================================================

/// Builder for configuring and connecting to an agtrace workspace.
///
/// Provides flexible path resolution with the following priority:
/// 1. Explicit path via `builder.path()`
/// 2. `AGTRACE_PATH` environment variable
/// 3. System data directory (e.g., `~/.local/share/agtrace` on Linux, `~/Library/Application Support/agtrace` on macOS)
///
/// # Examples
///
/// ```no_run
/// # use agtrace_sdk::Client;
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Use default system path
/// let client = Client::connect_default().await?;
///
/// // Use explicit path
/// let client = Client::builder()
///     .path("/custom/path")
///     .connect().await?;
///
/// // Use AGTRACE_PATH environment variable
/// // $ export AGTRACE_PATH=/tmp/agtrace
/// let client = Client::builder().connect().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Default)]
pub struct ClientBuilder {
    path: Option<PathBuf>,
}

impl ClientBuilder {
    /// Create a new ClientBuilder with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set an explicit workspace path (highest priority).
    pub fn path(mut self, path: impl Into<PathBuf>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Connect to the workspace using the configured or resolved path.
    /// If the workspace does not exist, it will be automatically initialized.
    pub async fn connect(self) -> Result<Client> {
        let path = self.resolve_path()?;
        let runtime = agtrace_runtime::AgTrace::connect_or_create(path)
            .await
            .map_err(Error::Runtime)?;
        Ok(Client {
            inner: Arc::new(runtime),
        })
    }

    /// Resolve the workspace path based on priority:
    /// 1. Explicit path from builder
    /// 2. AGTRACE_PATH environment variable
    /// 3. System data directory
    fn resolve_path(&self) -> Result<PathBuf> {
        let explicit_path = self.path.as_ref().and_then(|p| p.to_str());
        agtrace_runtime::resolve_workspace_path(explicit_path).map_err(Error::Runtime)
    }
}

// ============================================================================
// Main Client
// ============================================================================

/// Main entry point for interacting with an agtrace workspace.
#[derive(Clone)]
pub struct Client {
    inner: Arc<agtrace_runtime::AgTrace>,
}

impl Client {
    /// Create a new ClientBuilder for configuring workspace connection.
    ///
    /// This is the recommended way to connect to a workspace as it supports
    /// platform-standard path resolution and environment variable configuration.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use agtrace_sdk::Client;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// // Use default path
    /// let client = Client::builder().connect().await?;
    ///
    /// // Use custom path
    /// let client = Client::builder()
    ///     .path("/custom/agtrace")
    ///     .connect().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    /// Connect to the default agtrace workspace.
    ///
    /// This is a convenience method that uses platform-standard path resolution.
    /// It checks (in order):
    /// 1. `AGTRACE_PATH` environment variable
    /// 2. System data directory + "agtrace" (e.g., `~/.local/share/agtrace`)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use agtrace_sdk::Client;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::connect_default().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect_default() -> Result<Self> {
        Self::builder().connect().await
    }

    /// Connect to an agtrace workspace at the given path.
    ///
    /// This is a low-level API. Consider using `Client::builder()` or
    /// `Client::connect_default()` for better ergonomics and system path support.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use agtrace_sdk::Client;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::connect("/custom/agtrace/path").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect(path: impl Into<PathBuf>) -> Result<Self> {
        Self::builder().path(path).connect().await
    }

    /// Access session operations.
    pub fn sessions(&self) -> SessionClient {
        SessionClient {
            inner: self.inner.clone(),
        }
    }

    /// Access project operations.
    pub fn projects(&self) -> ProjectClient {
        ProjectClient {
            inner: self.inner.clone(),
        }
    }

    /// Access watch/monitoring operations.
    pub fn watch(&self) -> WatchClient {
        WatchClient {
            inner: self.inner.clone(),
        }
    }

    /// Access insights/analysis operations.
    pub fn insights(&self) -> InsightClient {
        InsightClient {
            inner: self.inner.clone(),
        }
    }

    /// Access system operations (init, index, doctor, provider).
    pub fn system(&self) -> SystemClient {
        SystemClient {
            inner: self.inner.clone(),
        }
    }

    /// Get the watch service for low-level watch operations.
    /// Prefer using `client.watch()` for most use cases.
    pub fn watch_service(&self) -> crate::types::WatchService {
        self.inner.watch_service()
    }
}

// ============================================================================
// SessionClient
// ============================================================================

/// Client for session-related operations.
pub struct SessionClient {
    inner: Arc<agtrace_runtime::AgTrace>,
}

impl SessionClient {
    /// List sessions with optional filtering.
    pub fn list(&self, filter: SessionFilter) -> Result<Vec<SessionSummary>> {
        self.inner.sessions().list(filter).map_err(Error::Runtime)
    }

    /// List sessions without triggering auto-refresh.
    pub fn list_without_refresh(&self, filter: SessionFilter) -> Result<Vec<SessionSummary>> {
        self.inner
            .sessions()
            .list_without_refresh(filter)
            .map_err(Error::Runtime)
    }

    /// Pack sessions for context window analysis.
    pub fn pack_context(
        &self,
        project_hash: Option<&crate::types::ProjectHash>,
        limit: usize,
    ) -> Result<crate::types::PackResult> {
        self.inner
            .sessions()
            .pack_context(project_hash, limit)
            .map_err(Error::Runtime)
    }

    /// Get a session handle by ID or prefix.
    pub fn get(&self, id_or_prefix: &str) -> Result<SessionHandle> {
        // Validate the session exists by trying to find it
        self.inner
            .sessions()
            .find(id_or_prefix)
            .map_err(|e| Error::NotFound(format!("Session {}: {}", id_or_prefix, e)))?;

        Ok(SessionHandle {
            source: SessionSource::Workspace {
                inner: self.inner.clone(),
                id: id_or_prefix.to_string(),
            },
        })
    }

    // ========================================================================
    // MCP Query Methods
    // ========================================================================

    /// Search events across sessions.
    pub fn search_events(&self, args: SearchEventsArgs) -> Result<SearchEventsResponse> {
        let limit = args.limit();
        let offset = args
            .cursor
            .as_ref()
            .and_then(|c| Cursor::decode(c))
            .map(|c| c.offset)
            .unwrap_or(0);

        let project_hash_filter = if let Some(ref root) = args.project_root {
            Some(crate::utils::project_hash_from_root(root))
        } else {
            args.project_hash.clone().map(|h| h.into())
        };

        let mut filter = if let Some(hash) = project_hash_filter {
            SessionFilter::project(hash).limit(1000)
        } else {
            SessionFilter::all().limit(1000)
        };

        if let Some(ref provider) = args.provider {
            filter = filter.provider(provider.as_str().to_string());
        }

        let sessions = if let Some(ref session_id) = args.session_id {
            let _handle = self.get(session_id)?;

            vec![SessionSummary {
                id: session_id.clone(),
                provider: String::new(),
                project_hash: ProjectHash::from(String::new()),
                project_root: None,
                start_ts: None,
                snippet: None,
                parent_session_id: None,
                spawned_by: None,
            }]
        } else {
            self.list_without_refresh(filter)?
        };

        let mut all_matches = Vec::new();

        for session_summary in sessions {
            let handle = match self.get(&session_summary.id) {
                Ok(h) => h,
                Err(_) => continue,
            };

            let session = match handle.assemble() {
                Ok(s) => s,
                Err(_) => continue,
            };

            let events = match handle.events() {
                Ok(e) => e,
                Err(_) => continue,
            };

            for (event_index, event) in events.iter().enumerate() {
                if let Some(ref event_type_filter) = args.event_type
                    && !event_type_filter.matches_payload(&event.payload)
                {
                    continue;
                }

                let event_json = match serde_json::to_string(&event.payload) {
                    Ok(j) => j,
                    Err(_) => continue,
                };

                if event_json.contains(&args.query) {
                    let (turn_index, step_index) = Self::find_event_location(&session, event_index);

                    let event_match = EventMatch::new(
                        session_summary.id.clone(),
                        event_index,
                        turn_index,
                        step_index,
                        event,
                    );
                    all_matches.push(event_match);
                }
            }
        }

        let fetch_limit = limit + 1;
        let mut matches: Vec<_> = all_matches
            .into_iter()
            .skip(offset)
            .take(fetch_limit)
            .collect();

        let has_more = matches.len() > limit;
        if has_more {
            matches.pop();
        }

        let next_cursor = if has_more {
            Some(
                Cursor {
                    offset: offset + limit,
                }
                .encode(),
            )
        } else {
            None
        };

        Ok(SearchEventsResponse {
            matches,
            next_cursor,
        })
    }

    /// List turns with metadata (no payload).
    pub fn list_turns(&self, args: ListTurnsArgs) -> Result<ListTurnsResponse> {
        let handle = self.get(&args.session_id)?;

        let session = handle.assemble()?;

        let limit = args.limit();
        let offset = args
            .cursor
            .as_ref()
            .and_then(|c| Cursor::decode(c))
            .map(|c| c.offset)
            .unwrap_or(0);

        let total_turns = session.turns.len();
        let remaining = total_turns.saturating_sub(offset);
        let has_more = remaining > limit;

        let next_cursor = if has_more {
            Some(
                Cursor {
                    offset: offset + limit,
                }
                .encode(),
            )
        } else {
            None
        };

        Ok(ListTurnsResponse::new(session, offset, limit, next_cursor))
    }

    /// Get specific turns with safety valves.
    pub fn get_turns(&self, args: GetTurnsArgs) -> Result<GetTurnsResponse> {
        let handle = self.get(&args.session_id)?;

        let session = handle.assemble()?;

        GetTurnsResponse::new(session, &args).map_err(Error::InvalidInput)
    }

    fn find_event_location(session: &AgentSession, event_index: usize) -> (usize, usize) {
        let mut current_event_idx = 0;

        for (turn_idx, turn) in session.turns.iter().enumerate() {
            for (step_idx, step) in turn.steps.iter().enumerate() {
                let step_event_count = Self::count_step_events(step);

                if current_event_idx + step_event_count > event_index {
                    return (turn_idx, step_idx);
                }

                current_event_idx += step_event_count;
            }
        }

        (0, 0)
    }

    fn count_step_events(step: &AgentStep) -> usize {
        let mut count = 0;

        if step.reasoning.is_some() {
            count += 1;
        }

        count += step.tools.len() * 2;

        if step.message.is_some() {
            count += 1;
        }

        count
    }
}

// ============================================================================
// SessionHandle
// ============================================================================

/// Handle to a specific session, providing access to its data.
pub struct SessionHandle {
    source: SessionSource,
}

enum SessionSource {
    /// Session from a workspace (Client-based)
    Workspace {
        inner: Arc<agtrace_runtime::AgTrace>,
        id: String,
    },
    /// Session from raw events (Standalone)
    Events {
        events: Vec<crate::types::AgentEvent>,
    },
}

impl SessionHandle {
    /// Create a SessionHandle from raw events (for testing, simulations, custom pipelines).
    ///
    /// This allows you to use SessionHandle API without a Client connection.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use agtrace_sdk::{SessionHandle, types::AgentEvent};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let events: Vec<AgentEvent> = vec![/* ... */];
    /// let handle = SessionHandle::from_events(events);
    ///
    /// let session = handle.assemble()?;
    /// let summary = handle.summarize()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_events(events: Vec<AgentEvent>) -> Self {
        Self {
            source: SessionSource::Events { events },
        }
    }

    /// Load raw events for this session.
    pub fn events(&self) -> Result<Vec<AgentEvent>> {
        match &self.source {
            SessionSource::Workspace { inner, id } => {
                let session_handle = inner
                    .sessions()
                    .find(id)
                    .map_err(|e| Error::NotFound(format!("Session {}: {}", id, e)))?;

                session_handle.events().map_err(Error::Runtime)
            }
            SessionSource::Events { events } => Ok(events.clone()),
        }
    }

    /// Assemble events into a structured session.
    ///
    /// Returns only the main stream. For multi-stream sessions (with sidechains
    /// or subagents), use `assemble_all()` instead.
    pub fn assemble(&self) -> Result<AgentSession> {
        let events = self.events()?;
        agtrace_engine::assemble_session(&events).ok_or_else(|| {
            Error::InvalidInput(
                "Failed to assemble session: insufficient or invalid events".to_string(),
            )
        })
    }

    /// Assemble all streams from events into separate sessions.
    ///
    /// Unlike `assemble()` which returns only the main stream, this method
    /// returns all streams (Main, Sidechain, Subagent) found in the session's events.
    pub fn assemble_all(&self) -> Result<Vec<AgentSession>> {
        let events = self.events()?;
        let sessions = agtrace_engine::assemble_sessions(&events);
        if sessions.is_empty() {
            return Err(Error::InvalidInput(
                "Failed to assemble session: insufficient or invalid events".to_string(),
            ));
        }
        Ok(sessions)
    }

    /// Export session with specified strategy.
    pub fn export(&self, strategy: ExportStrategy) -> Result<Vec<AgentEvent>> {
        let events = self.events()?;
        Ok(agtrace_engine::export::transform(&events, strategy))
    }

    /// Get session metadata (DB-derived: project_hash, provider).
    ///
    /// Returns None for standalone sessions (created from events without workspace).
    pub fn metadata(&self) -> Result<Option<crate::types::SessionMetadata>> {
        match &self.source {
            SessionSource::Workspace { inner, id } => {
                let runtime_handle = inner
                    .sessions()
                    .find(id)
                    .map_err(|e| Error::NotFound(format!("Session {}: {}", id, e)))?;

                runtime_handle.metadata().map(Some).map_err(Error::Runtime)
            }
            SessionSource::Events { .. } => Ok(None),
        }
    }

    /// Get raw log files for this session.
    ///
    /// Returns the list of raw log file paths and their contents.
    /// Returns empty vector for standalone sessions (created from events without workspace).
    pub fn raw_files(&self) -> Result<Vec<crate::types::RawFileContent>> {
        match &self.source {
            SessionSource::Workspace { inner, id } => {
                let runtime_handle = inner
                    .sessions()
                    .find(id)
                    .map_err(|e| Error::NotFound(format!("Session {}: {}", id, e)))?;

                runtime_handle.raw_files().map_err(Error::Runtime)
            }
            SessionSource::Events { .. } => Ok(vec![]),
        }
    }

    /// Summarize session statistics.
    pub fn summarize(&self) -> Result<agtrace_engine::SessionSummary> {
        let session = self.assemble()?;
        Ok(agtrace_engine::session::summarize(&session))
    }

    /// Analyze session with diagnostic lenses.
    pub fn analyze(&self) -> Result<crate::analysis::SessionAnalyzer> {
        let session = self.assemble()?;
        Ok(crate::analysis::SessionAnalyzer::new(session))
    }

    /// Get child sessions (subagents) that were spawned from this session.
    ///
    /// Returns a list of child session summaries with their spawn context
    /// (turn_index, step_index). Returns empty vector for standalone sessions.
    pub fn child_sessions(&self) -> Result<Vec<ChildSessionInfo>> {
        match &self.source {
            SessionSource::Workspace { inner, id } => {
                let runtime_handle = inner
                    .sessions()
                    .find(id)
                    .map_err(|e| Error::NotFound(format!("Session {}: {}", id, e)))?;

                let children = runtime_handle.child_sessions().map_err(Error::Runtime)?;
                Ok(children
                    .into_iter()
                    .map(|c| ChildSessionInfo {
                        session_id: c.id,
                        provider: c.provider,
                        spawned_by: c.spawned_by,
                        snippet: c.snippet,
                    })
                    .collect())
            }
            SessionSource::Events { .. } => Ok(vec![]),
        }
    }
}

/// Information about a child session (subagent) spawned from a parent session.
#[derive(Debug, Clone)]
pub struct ChildSessionInfo {
    pub session_id: String,
    pub provider: String,
    pub spawned_by: Option<agtrace_types::SpawnContext>,
    pub snippet: Option<String>,
}

// ============================================================================
// ProjectClient
// ============================================================================

/// Client for project-related operations.
pub struct ProjectClient {
    inner: Arc<agtrace_runtime::AgTrace>,
}

impl ProjectClient {
    /// List all projects in the workspace.
    pub fn list(&self) -> Result<Vec<ProjectInfo>> {
        self.inner.projects().list().map_err(Error::Runtime)
    }
}

// ============================================================================
// WatchClient
// ============================================================================

/// Client for live monitoring operations.
pub struct WatchClient {
    inner: Arc<agtrace_runtime::AgTrace>,
}

impl WatchClient {
    /// Create a watch builder for configuring monitoring.
    pub fn builder(&self) -> WatchBuilder {
        WatchBuilder::new(self.inner.clone())
    }

    /// Watch all providers (convenience method).
    pub fn all_providers(&self) -> WatchBuilder {
        WatchBuilder::new(self.inner.clone()).all_providers()
    }

    /// Watch a specific provider (convenience method).
    pub fn provider(&self, name: &str) -> WatchBuilder {
        WatchBuilder::new(self.inner.clone()).provider(name)
    }

    /// Watch a specific session (convenience method).
    pub fn session(&self, _id: &str) -> WatchBuilder {
        // WatchBuilder doesn't have a session method yet, return builder for now
        WatchBuilder::new(self.inner.clone())
    }
}

// ============================================================================
// InsightClient
// ============================================================================

/// Client for analysis and insights operations.
pub struct InsightClient {
    inner: Arc<agtrace_runtime::AgTrace>,
}

impl InsightClient {
    /// Get corpus statistics.
    pub fn corpus_stats(
        &self,
        project_hash: Option<&agtrace_types::ProjectHash>,
        limit: usize,
    ) -> Result<CorpusStats> {
        self.inner
            .insights()
            .corpus_stats(project_hash, limit)
            .map_err(Error::Runtime)
    }

    /// Get tool usage statistics.
    pub fn tool_usage(
        &self,
        limit: Option<usize>,
        provider: Option<String>,
    ) -> Result<agtrace_runtime::StatsResult> {
        self.inner
            .insights()
            .tool_usage(limit, provider)
            .map_err(Error::Runtime)
    }

    /// Pack sessions for analysis (placeholder - needs runtime implementation).
    pub fn pack(&self, _limit: usize) -> Result<PackResult> {
        // TODO: This needs to be implemented in agtrace-runtime
        Err(Error::InvalidInput(
            "Pack operation not yet implemented in runtime".to_string(),
        ))
    }

    /// Grep through tool calls (placeholder - needs runtime implementation).
    pub fn grep(
        &self,
        _pattern: &str,
        _filter: &SessionFilter,
        _limit: usize,
    ) -> Result<Vec<AgentEvent>> {
        // TODO: This needs to be implemented in agtrace-runtime
        Err(Error::InvalidInput(
            "Grep operation not yet implemented in runtime".to_string(),
        ))
    }
}

// ============================================================================
// SystemClient
// ============================================================================

/// Client for system-level operations (init, index, doctor, provider).
pub struct SystemClient {
    inner: Arc<agtrace_runtime::AgTrace>,
}

impl SystemClient {
    /// Initialize a new workspace (static method).
    pub fn initialize<F>(config: InitConfig, on_progress: Option<F>) -> Result<InitResult>
    where
        F: FnMut(InitProgress),
    {
        agtrace_runtime::AgTrace::setup(config, on_progress).map_err(Error::Runtime)
    }

    /// Run diagnostics on all providers.
    pub fn diagnose(&self) -> Result<Vec<DiagnoseResult>> {
        self.inner.diagnose().map_err(Error::Runtime)
    }

    /// Check if a file can be parsed (requires workspace context).
    pub fn check_file(&self, path: &Path, provider: Option<&str>) -> Result<CheckResult> {
        let path_str = path
            .to_str()
            .ok_or_else(|| Error::InvalidInput("Path contains invalid UTF-8".to_string()))?;

        // Detect adapter
        let (adapter, provider_name) = if let Some(name) = provider {
            let adapter = agtrace_providers::create_adapter(name)
                .map_err(|_| Error::NotFound(format!("Provider: {}", name)))?;
            (adapter, name.to_string())
        } else {
            let adapter = agtrace_providers::detect_adapter_from_path(path_str)
                .map_err(|_| Error::NotFound("No suitable provider detected".to_string()))?;
            let name = format!("{} (auto-detected)", adapter.id());
            (adapter, name)
        };

        agtrace_runtime::AgTrace::check_file(path_str, &adapter, &provider_name)
            .map_err(Error::Runtime)
    }

    /// Inspect file contents with parsing.
    pub fn inspect_file(path: &Path, lines: usize, json_format: bool) -> Result<InspectResult> {
        let path_str = path
            .to_str()
            .ok_or_else(|| Error::InvalidInput("Path contains invalid UTF-8".to_string()))?;

        agtrace_runtime::AgTrace::inspect_file(path_str, lines, json_format).map_err(Error::Runtime)
    }

    /// Reindex the workspace.
    pub fn reindex<F>(
        &self,
        scope: agtrace_types::ProjectScope,
        force: bool,
        provider_filter: Option<&str>,
        on_progress: F,
    ) -> Result<()>
    where
        F: FnMut(IndexProgress),
    {
        self.inner
            .projects()
            .scan(scope, force, provider_filter, on_progress)
            .map(|_| ()) // Discard the ScanSummary for now
            .map_err(Error::Runtime)
    }

    /// Vacuum the database to reclaim space.
    pub fn vacuum(&self) -> Result<()> {
        let db = self.inner.database();
        let db = db.lock().unwrap();
        db.vacuum().map_err(|e| Error::Runtime(e.into()))
    }

    /// List provider configurations.
    pub fn list_providers(&self) -> Result<Vec<ProviderConfig>> {
        Ok(self.inner.config().providers.values().cloned().collect())
    }

    /// Detect providers in current environment.
    pub fn detect_providers() -> Result<Config> {
        agtrace_runtime::Config::detect_providers().map_err(Error::Runtime)
    }

    /// Get current configuration.
    pub fn config(&self) -> Config {
        self.inner.config().clone()
    }
}
