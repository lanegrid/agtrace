//! Type re-exports for the SDK.
//!
//! This module re-exports all types that SDK consumers (like the CLI) need to work with.
//! By centralizing these re-exports, we maintain a stable API boundary while allowing
//! internal crate refactoring without breaking SDK clients.

// ============================================================================
// Event Types (from agtrace-types)
// ============================================================================

// Note: agtrace-types re-exports everything at the top level from domain, event, and tool modules
pub use agtrace_types::{
    // Event types
    AgentEvent,
    EventPayload,
    // Payload types
    ExecuteArgs,
    FileEditArgs,
    FileReadArgs,
    MessagePayload,
    // Domain types
    ProjectHash,
    ProjectScope,
    ReasoningPayload,
    StreamId,
    TokenUsagePayload,
    ToolCallPayload,
    ToolKind,
    ToolResultPayload,
    UserPayload,
    // Utility functions are also re-exported at top level
    discover_project_root,
    project_hash_from_root,
    resolve_effective_project_hash,
};

// ============================================================================
// Session Types (from agtrace-engine)
// ============================================================================

pub use agtrace_engine::{
    // Session assembly types
    AgentSession,
    AgentStep,
    AgentTurn,
    // Token usage types
    ContextLimit,
    ContextWindowUsage,
    // Analysis types
    SessionDigest,
    SessionStats,
    TokenCount,
    ToolExecution,
    TurnMetrics,
    // Functions
    assemble_session,
    extract_state_updates,
};

// Re-export summarize function with clearer name
pub use agtrace_engine::session::summary::summarize as summarize_session;

// ============================================================================
// Export Strategy (from agtrace-engine)
// ============================================================================

pub use agtrace_engine::export::ExportStrategy;

// ============================================================================
// Diagnostics Types (from agtrace-engine)
// ============================================================================

pub use agtrace_engine::{DiagnoseResult, FailureExample, FailureType};

// ============================================================================
// Index Types (from agtrace-index)
// ============================================================================

pub use agtrace_index::SessionSummary;

// ============================================================================
// Runtime Types (from agtrace-runtime)
// ============================================================================

pub use agtrace_runtime::{
    // Operations Types
    CheckResult,
    CheckStatus,
    // Configuration Types
    Config,
    // Init Types
    ConfigStatus,
    CorpusStats,
    // Watch/Monitor Types
    DiscoveryEvent,
    // Event Filters
    EventFilters,
    IndexProgress,
    InitConfig,
    InitProgress,
    InitResult,
    InspectContentType,
    InspectLine,
    InspectResult,
    PackResult,
    ProjectInfo,
    ProviderConfig,
    ScanOutcome,
    // Session Filter
    SessionFilter,
    SessionState,
    StatsResult,
    StreamEvent,
    StreamHandle,
    TokenLimit,
    TokenLimits,
    WatchService,
    WorkspaceEvent,
};

// ============================================================================
// Provider Types (from agtrace-providers)
// ============================================================================

// Re-export provider detection utilities
pub use agtrace_providers::{create_adapter, detect_adapter_from_path};
