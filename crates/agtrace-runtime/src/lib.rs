pub mod client;
pub mod config;
pub mod domain;
pub mod init;
pub mod ops;
pub mod runtime;
pub mod storage;

// New workspace-oriented interface (recommended)
pub use client::{
    ActiveRuntime, AgTrace, InsightOps, ProjectOps, RuntimeBuilder, SessionFilter, SessionHandle,
    SessionOps,
};

// Legacy exports (for backward compatibility)
pub use config::{Config, ProviderConfig};
pub use domain::{filter_events, EventFilters, SessionState, TokenLimit, TokenLimits};
pub use init::{
    ConfigStatus, InitConfig, InitProgress, InitResult, InitService, ProviderInfo, ScanOutcome,
};
pub use ops::{
    collect_tool_stats, get_corpus_overview, CheckResult, CheckStatus, CorpusStats, DoctorService,
    ExportService, IndexProgress, IndexService, InspectContentType, InspectLine, InspectResult,
    ListSessionsRequest, PackResult, PackService, ProjectInfo, ProjectService, ProviderStats,
    SessionService, StatsResult, ToolInfo, ToolSample,
};
pub use runtime::{
    Reaction, Reactor, ReactorContext, Runtime, RuntimeConfig, RuntimeEvent, SessionUpdate,
    SessionWatcher, TokenUsageMonitor, WatchConfig, WatchEvent, WatchService,
};
pub use storage::{get_raw_files, LoadOptions, RawFileContent, SessionRepository};
