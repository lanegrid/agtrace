// Internal modules (not exposed to external crates)
pub(crate) mod config;
pub(crate) mod init;
pub(crate) mod model_catalog;
pub(crate) mod ops;
pub(crate) mod runtime;
pub(crate) mod storage;
pub(crate) mod tail;
pub(crate) mod workspace;

// Error types
pub mod error;

// Public client interface
pub mod client;

// Main facade and operations (workspace-oriented interface)
pub use client::{
    AgTrace, InsightOps, MonitorBuilder, ProjectOps, SessionFilter, SessionHandle, SessionOps,
    StreamHandle, WatchService, WorkspaceMonitor,
};
pub use runtime::SessionStreamer;
pub use workspace::{
    ProcessStatus, RescanHandle, SideStateUpdate, TeamMember, WatchRoots, WatchScope,
    WatcherOptions, WorkspaceEvent, WorkspaceWatcher,
};

// Data types used as inputs/outputs in public APIs
pub use agtrace_engine::{EventFilters, SessionState, filter_events};
pub use config::{Config, ContextWindowConfig, ProviderConfig, resolve_workspace_path};
pub use init::{ConfigStatus, InitConfig, InitProgress, InitResult, ScanOutcome};
pub use model_catalog::ConfiguredModelCatalog;
pub use ops::{
    CheckResult, CheckStatus, CorpusStats, DoctorService, IndexProgress, InspectContentType,
    InspectLine, InspectResult, PackResult, ProjectInfo, StatsResult,
};
pub use runtime::{DiscoveryEvent, StreamEvent, WatchEvent};
pub use storage::RawFileContent;

// Error types
pub use error::{Error, Result};
