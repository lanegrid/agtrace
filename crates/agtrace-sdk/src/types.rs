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
    // Agent identity & multi-agent payloads
    AgentAttributeKey,
    AgentAttributePayload,
    // Event types
    AgentEvent,
    AgentHandle,
    AgentId,
    AgentKind,
    AgentLifecyclePayload,
    AgentMessageKind,
    AgentMessagePayload,
    AgentOp,
    AgentRef,
    AgentRunUsage,
    // Session assembly types (moved from agtrace-engine)
    AgentSession,
    AgentSpawnPayload,
    AgentStep,
    AgentToolArgs,
    AgentTurn,
    CompactionPayload,
    CompactionTrigger,
    // Context window resolution
    ContextSource,
    ContextWindow,
    ContextWindowHintPayload,
    EventOrigin,
    EventPayload,
    // Payload types
    ExecuteArgs,
    FileEditArgs,
    FileReadArgs,
    LifecycleTransition,
    LineError,
    MessageBlock,
    MessageDirection,
    MessagePayload,
    ModelCatalog,
    ModelChangePayload,
    ModelChangeSource,
    ParseDiagnostics,
    PlanItem,
    PlanItemStatus,
    PlanPayload,
    // Domain types
    ProjectHash,
    ProjectScope,
    QueueOperationPayload,
    ReasoningBlock,
    ReasoningPayload,
    RepositoryHash,
    SessionMetadata,
    SessionStats,
    StepStatus,
    SubActionStatus,
    // Token usage types
    TokenInput,
    TokenOutput,
    TokenUsagePayload,
    ToolCallBlock,
    ToolCallPayload,
    ToolExecution,
    ToolKind,
    ToolResultBlock,
    ToolResultPayload,
    ToolSubActionPayload,
    TurnEndPayload,
    TurnMetrics,
    TurnOutcome,
    TurnStats,
    UsageCompleteness,
    UserMessage,
    UserPayload,
    // Utility functions
    truncate,
};

// ============================================================================
// Session Analysis Types (from agtrace-engine)
// ============================================================================

/// Provider of an agent log (agent-identity level; distinct from the query filter
/// [`crate::Provider`]).
pub use agtrace_types::Provider as AgentProvider;

pub use agtrace_engine::{
    // Context window evidence (fold events, then resolve)
    ContextEvidence,
    // Token usage types
    ContextLimit,
    ContextWindowUsage,
    // Extension traits
    SessionAnalysisExt,
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
    // Agent tree of an indexed session
    AgentNode,
    // Operations Types
    CheckResult,
    CheckStatus,
    // Configuration Types
    Config,
    // Init Types
    ConfigStatus,
    ConfiguredModelCatalog,
    ContextWindowConfig,
    CorpusStats,
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
    // Storage Types
    RawFileContent,
    ScanOutcome,
    // Session Filter
    SessionFilter,
    StatsResult,
};

// Workspace watcher (design §4.2 / §4.4)
pub use crate::watch::{
    LiveWorkspace, ProcessStatus, SideStateUpdate, TeamMember, WatchRoots, WatchScope,
    WatcherOptions, WorkspaceEvent, WorkspaceView,
};

// ============================================================================
// Provider Types (from agtrace-providers)
// ============================================================================

// Note: Provider adapter functions are now internal implementation details.
// External users should use SystemClient::check_file() instead.
