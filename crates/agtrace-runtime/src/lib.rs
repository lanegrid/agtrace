// Internal modules (not exposed to external crates)
pub(crate) mod config;
pub(crate) mod init;
pub(crate) mod ops;
pub(crate) mod runtime;
pub(crate) mod storage;

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

// Data types used as inputs/outputs in public APIs
pub use agtrace_engine::{EventFilters, SessionState, TokenLimit, TokenLimits, filter_events};
pub use config::{Config, ProviderConfig, resolve_workspace_path};
pub use init::{ConfigStatus, InitConfig, InitProgress, InitResult, ScanOutcome};
pub use ops::{
    CheckResult, CheckStatus, CorpusStats, DoctorService, IndexProgress, InspectContentType,
    InspectLine, InspectResult, PackResult, ProjectInfo, StatsResult,
};
pub use runtime::{DiscoveryEvent, StreamEvent, WorkspaceEvent};
pub use storage::RawFileContent;

// Error types
pub use error::{Error, Result};
