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
    // Session assembly types (moved from agtrace-engine)
    AgentSession,
    AgentStep,
    AgentTurn,
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
    SessionStats,
    StreamId,
    TokenUsagePayload,
    ToolCallPayload,
    ToolExecution,
    ToolKind,
    ToolResultPayload,
    TurnMetrics,
    UserPayload,
};

// ============================================================================
// Session Analysis Types (from agtrace-engine)
// ============================================================================

pub use agtrace_engine::{
    // Token usage types
    ContextLimit,
    ContextWindowUsage,
    // Analysis types
    SessionDigest,
    TokenCount,
};

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

// Note: Provider adapter functions are now internal implementation details.
// External users should use SystemClient::check_file() instead.
