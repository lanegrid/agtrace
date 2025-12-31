use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::error::{Error, Result};
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
/// 3. XDG data directory (e.g., `~/.local/share/agtrace` on Linux, `~/Library/Application Support/agtrace` on macOS)
///
/// # Examples
///
/// ```no_run
/// # use agtrace_sdk::Client;
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Use default XDG path
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
    /// 3. XDG data directory
    fn resolve_path(&self) -> Result<PathBuf> {
        let explicit_path = self.path.as_ref().and_then(|p| p.to_str());
        agtrace_runtime::resolve_workspace_path(explicit_path).map_err(Error::Runtime)
    }
}

// ============================================================================
// Main Client
// ============================================================================

/// Main entry point for interacting with an agtrace workspace.
pub struct Client {
    inner: Arc<agtrace_runtime::AgTrace>,
}

impl Client {
    /// Create a new ClientBuilder for configuring workspace connection.
    ///
    /// This is the recommended way to connect to a workspace as it supports
    /// XDG-compliant path resolution and environment variable configuration.
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
    /// This is a convenience method that uses XDG-compliant path resolution.
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
    /// `Client::connect_default()` for better ergonomics and XDG support.
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
    pub fn assemble(&self) -> Result<AgentSession> {
        let events = self.events()?;
        agtrace_engine::assemble_session(&events).ok_or_else(|| {
            Error::InvalidInput(
                "Failed to assemble session: insufficient or invalid events".to_string(),
            )
        })
    }

    /// Export session with specified strategy.
    pub fn export(&self, strategy: ExportStrategy) -> Result<Vec<AgentEvent>> {
        let events = self.events()?;
        Ok(agtrace_engine::export::transform(&events, strategy))
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
