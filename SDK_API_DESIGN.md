# SDK API Design for CLI Refactoring

This document defines the complete SDK API surface required to make the CLI depend only on `agtrace-sdk`.

## Current State Analysis

### CLI Dependencies (from Cargo.toml)
- agtrace-types
- agtrace-providers
- agtrace-index
- agtrace-engine
- agtrace-runtime

### CLI Import Analysis

Based on scanning `crates/agtrace-cli/src/`, the following types and functions are imported:

#### From agtrace-types
- `AgentEvent` - Event data structure (used in lab, watch, services)
- `EventPayload` - Event payload variants (used in lab, services)
- `StreamId` - Stream identifier (used in lab view models)
- `discover_project_root` - Utility function (used in project handler)
- `resolve_effective_project_hash` - Utility function (used in pack handler)

#### From agtrace-engine
- `assemble_session` - Session assembly function (used in demo, session_show)
- `AgentSession` - Assembled session structure (used in watch_tui, presenters)
- `SessionDigest` - Pack digest type (used in pack presenter)
- `ExportStrategy` - Export strategy enum (used in lab_export)

#### From agtrace-index
- `SessionSummary` - Session summary type (used in session presenter)

#### From agtrace-runtime
- `AgTrace` - Main runtime client (used in ALL handlers)
- `SessionState` - Live session state (used in demo, watch_console, watch_tui)
- `WorkspaceEvent` - Workspace event type (used in watch handlers)
- `DiscoveryEvent` - Discovery event type (used in watch handlers)
- `StreamEvent` - Stream event type (used in watch handlers)
- `SessionFilter` - Session filtering (used in lab_grep, session_list, session_show)
- `TokenLimits` - Token limit info (used in session_show)
- `IndexProgress` - Indexing progress (used in index handler)
- `InitConfig` - Initialization config (used in init handler)
- `Config` - Runtime config (used in provider handler)
- `ProviderConfig` - Provider config (used in provider handler)
- `ProjectInfo` - Project info (used in project presenter)

#### From agtrace-providers
- `create_adapter` - Create provider adapter (used in doctor_check)
- `detect_adapter_from_path` - Detect adapter (used in doctor_check)

## SDK Module Structure

### Module: `agtrace_sdk::types`

Re-export all domain types that CLI needs to work with:

```rust
// Event types (from agtrace-types)
pub use agtrace_types::{
    AgentEvent,
    EventPayload,
    StreamId,
    ProjectHash,
    ToolKind,
    ToolCallPayload,
    ToolResultPayload,
    UserPayload,
    MessagePayload,
    ReasoningPayload,
    TokenUsagePayload,
    NotificationPayload,
};

// Utility functions (from agtrace-types)
pub use agtrace_types::{
    discover_project_root,
    resolve_effective_project_hash,
    project_hash_from_root,
};

// Session types (from agtrace-engine)
pub use agtrace_engine::{
    AgentSession,
    AgentTurn,
    AgentStep,
    SessionDigest,
    TurnMetrics,
    ContextWindowUsage,
};

// Index types (from agtrace-index)
pub use agtrace_index::SessionSummary;

// Export strategy (from agtrace-engine)
pub use agtrace_engine::export::ExportStrategy;

// Watch event types (from agtrace-runtime)
pub use agtrace_runtime::{
    SessionState,
    WorkspaceEvent,
    DiscoveryEvent,
    StreamEvent,
    TokenLimits,
    TokenLimit,
};

// Filter and config types (from agtrace-runtime)
pub use agtrace_runtime::{
    SessionFilter,
    Config,
    ProviderConfig,
    ProjectInfo,
    InitConfig,
    IndexProgress,
};
```

**Rationale**: These are pure data types that the CLI uses for display, filtering, and configuration. Re-exporting them allows CLI to access them without depending on internal crates directly.

### Module: `agtrace_sdk::client`

Main entry point and operation facades:

```rust
pub struct Client {
    inner: Arc<agtrace_runtime::AgTrace>,
}

impl Client {
    /// Connect to an agtrace workspace
    pub fn connect(path: impl Into<PathBuf>) -> Result<Self>;

    /// Access session operations
    pub fn sessions(&self) -> SessionClient;

    /// Access project operations
    pub fn projects(&self) -> ProjectClient;

    /// Access watch/monitoring operations
    pub fn watch(&self) -> WatchClient;

    /// Access insights/analysis operations
    pub fn insights(&self) -> InsightClient;

    /// Access system operations (init, index, doctor, provider)
    pub fn system(&self) -> SystemClient;
}
```

#### SessionClient

```rust
pub struct SessionClient {
    inner: Arc<agtrace_runtime::AgTrace>,
}

impl SessionClient {
    /// List sessions with optional filtering
    pub fn list(&self, filter: SessionFilter) -> Result<Vec<SessionSummary>>;

    /// Get a session handle by ID or prefix
    pub fn get(&self, id_or_prefix: &str) -> Result<SessionHandle>;
}

pub struct SessionHandle {
    id: String,
    inner: Arc<agtrace_runtime::AgTrace>,
}

impl SessionHandle {
    /// Load raw events for this session
    pub fn events(&self) -> Result<Vec<AgentEvent>>;

    /// Assemble events into a structured session
    pub fn assemble(&self) -> Result<AgentSession>;

    /// Get raw file contents for this session
    pub fn raw_files(&self) -> Result<Vec<RawFileContent>>;

    /// Export session with specified strategy
    pub fn export(&self, strategy: ExportStrategy) -> Result<Vec<AgentEvent>>;
}
```

#### ProjectClient

```rust
pub struct ProjectClient {
    inner: Arc<agtrace_runtime::AgTrace>,
}

impl ProjectClient {
    /// List all projects in the workspace
    pub fn list(&self) -> Result<Vec<ProjectInfo>>;
}
```

#### WatchClient

```rust
pub struct WatchClient {
    inner: Arc<agtrace_runtime::AgTrace>,
    provider_filter: Option<String>,
    session_filter: Option<String>,
}

impl WatchClient {
    /// Filter by provider name
    pub fn provider(mut self, name: &str) -> Self;

    /// Filter by session ID
    pub fn session(mut self, id: &str) -> Self;

    /// Watch all providers
    pub fn all_providers(mut self) -> Self;

    /// Start monitoring and return event stream
    pub fn start(self) -> Result<LiveStream>;
}
```

**Note**: `LiveStream` already exists in `agtrace-sdk/src/watch.rs`

#### InsightClient

```rust
pub struct InsightClient {
    inner: Arc<agtrace_runtime::AgTrace>,
}

impl InsightClient {
    /// Get corpus statistics
    pub fn corpus_stats(&self, filter: &SessionFilter) -> Result<CorpusStats>;

    /// Get tool usage statistics
    pub fn tool_usage(&self, limit: Option<usize>) -> Result<Vec<ToolUsageStat>>;

    /// Pack sessions for analysis
    pub fn pack(&self, limit: usize, strategy: PackStrategy) -> Result<Vec<SessionDigest>>;

    /// Grep through tool calls
    pub fn grep(&self,
                pattern: &str,
                tool_filter: Option<ToolKind>,
                limit: usize) -> Result<Vec<ToolMatch>>;
}
```

#### SystemClient

```rust
pub struct SystemClient {
    inner: Arc<agtrace_runtime::AgTrace>,
}

impl SystemClient {
    /// Initialize a new workspace (static method)
    pub fn initialize(
        config: InitConfig,
        on_progress: impl Fn(InitProgress),
    ) -> Result<InitResult>;

    /// Run diagnostics on all providers
    pub fn diagnose(&self) -> Result<Vec<DiagnoseResult>>;

    /// Check if a file can be parsed
    pub fn check_file(&self, path: &Path, provider: Option<&str>) -> Result<CheckResult>;

    /// Inspect file contents with parsing
    pub fn inspect_file(&self, path: &Path, lines: usize) -> Result<InspectResult>;

    /// Reindex the workspace
    pub fn reindex(
        &self,
        scope: ProjectScope,
        force: bool,
        provider_filter: Option<&str>,
        on_progress: impl Fn(IndexProgress),
    ) -> Result<ScanSummary>;

    /// Vacuum/cleanup database
    pub fn vacuum(&self) -> Result<()>;

    /// List provider configurations
    pub fn list_providers(&self) -> Result<Vec<ProviderConfig>>;

    /// Detect providers in current environment
    pub fn detect_providers(&self) -> Result<Config>;

    /// Get current configuration
    pub fn config(&self) -> &Config;
}
```

### Module: `agtrace_sdk::error`

Unified error type (already exists):

```rust
pub struct Error { /* ... */ }
pub type Result<T> = std::result::Result<T, Error>;
```

### Top-level Re-exports

In `agtrace_sdk::lib.rs`:

```rust
// Core client
pub use client::Client;

// Session types
pub use client::{SessionClient, SessionHandle, ProjectClient};

// Watch types
pub use watch::{WatchClient, LiveStream};

// Analysis types
pub use client::{InsightClient, SystemClient};

// Error types
pub use error::{Error, Result};

// Helper functions (from agtrace-engine)
pub use agtrace_engine::assemble_session;
pub use agtrace_engine::session::summarize;

// Convenience type re-exports (most common types)
pub use types::{
    AgentEvent,
    EventPayload,
    AgentSession,
    SessionSummary,
    SessionFilter,
    ExportStrategy,
};
```

## Migration Strategy by Handler

### Simple Handlers (Milestone 4)
- `provider.rs`: Use `client.system().list_providers()`, `client.system().detect_providers()`
- `project.rs`: Use `client.projects().list()`
- `doctor_run.rs`: Use `client.system().diagnose()`
- `doctor_check.rs`: Use `client.system().check_file()`
- `doctor_inspect.rs`: Use `client.system().inspect_file()`

### Session Handlers (Milestone 5)
- `session_list.rs`: Use `client.sessions().list(filter)`
- `session_show.rs`: Use `client.sessions().get(id)?.assemble()?`
- `lab_export.rs`: Use `client.sessions().get(id)?.export(strategy)?`

### Watch/TUI Handlers (Milestone 6)
- `watch_tui.rs`: Use `client.watch().provider(name).start()?`
- `watch_console.rs`: Use `client.watch().session(id).start()?`
- `demo.rs`: Use `client.watch()` and SDK types

### Init/Index Handlers (Milestone 7)
- `init.rs`: Use `SystemClient::initialize(config, on_progress)`
- `index.rs`: Use `client.system().reindex(scope, force, provider, on_progress)`

### Lab/Insight Handlers (Milestone 8)
- `lab_stats.rs`: Use `client.insights().corpus_stats(filter)`
- `pack.rs`: Use `client.insights().pack(limit, strategy)`
- `lab_grep.rs`: Use `client.insights().grep(pattern, tool_filter, limit)`

## Implementation Order

1. **Milestone 2**: Create `types.rs` module with all re-exports
2. **Milestone 3**: Implement `Client` and all sub-clients with method stubs
3. **Milestone 4-8**: Implement methods incrementally, migrating handlers in parallel
4. **Milestone 9**: Remove internal dependencies from CLI Cargo.toml

## Type Compatibility Notes

- `SessionState` must remain fully public (fields accessible) for TUI rendering
- `WorkspaceEvent`, `DiscoveryEvent`, `StreamEvent` must be re-exported for watch handlers
- `SessionFilter` construction must be possible from CLI (public fields or builder)
- `ExportStrategy` must be constructible from CLI args
- Error types must convert to `anyhow::Error` for CLI convenience

## Success Criteria

After refactoring:
- CLI `Cargo.toml` has NO dependencies on `agtrace-{types,engine,index,providers,runtime}`
- CLI `Cargo.toml` has ONLY `agtrace-sdk` as internal dependency
- All CLI commands produce identical output
- All tests pass
- No performance degradation (>10%) in any command
- `cargo build --workspace` succeeds
- `cargo clippy --workspace` has no new warnings
