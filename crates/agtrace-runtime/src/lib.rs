// Internal modules (not exposed to external crates)
pub(crate) mod config;
pub(crate) mod domain;
pub(crate) mod init;
pub(crate) mod ops;
pub(crate) mod runtime;
pub(crate) mod storage;

// Public client interface
pub mod client;

// Main facade and operations (workspace-oriented interface)
pub use client::{
    AgTrace, InsightOps, MonitorBuilder, ProjectOps, SessionFilter, SessionHandle, SessionOps,
    StreamHandle, WatchService, WorkspaceMonitor,
};
pub use runtime::SessionStreamer;

// Data types used as inputs/outputs in public APIs
pub use config::{Config, ProviderConfig};
pub use domain::{EventFilters, SessionState, TokenLimit, TokenLimits, filter_events};
pub use init::{ConfigStatus, InitConfig, InitProgress, InitResult, ScanOutcome};
pub use ops::{
    CheckResult, CheckStatus, CorpusStats, IndexProgress, InspectContentType, InspectLine,
    InspectResult, PackResult, ProjectInfo, StatsResult,
};
pub use runtime::{DiscoveryEvent, StreamEvent, WorkspaceEvent};
