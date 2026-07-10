use chrono::{DateTime, Utc};
use serde::Serialize;
use std::fmt;

use super::{CreateView, ViewMode};

// --------------------------------------------------------
// Data Definitions (ViewModels)
// --------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct SessionListViewModel {
    pub sessions: Vec<SessionListEntry>,
    pub total_count: usize,
    pub applied_filters: FilterSummary,
}

#[derive(Debug, Serialize)]
pub struct SessionListEntry {
    pub id: String,
    pub provider: String,
    pub project_hash: String,
    pub project_root: Option<String>,
    pub start_ts: Option<String>,
    pub snippet: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FilterSummary {
    pub project_filter: Option<String>,
    pub provider_filter: Option<String>,
    pub time_range: Option<String>,
    pub limit: usize,
}

/// Session detail view - a single document covering the whole session.
///
/// A session may contain multiple event streams (the main conversation plus
/// sidechains/subagents). They are all embedded in `streams` so that one
/// `session show` invocation always produces exactly one document, regardless
/// of output format.
#[derive(Debug, Serialize)]
pub struct SessionDetailViewModel {
    pub session: SessionInfoViewModel,
    /// All streams in this session. The main stream comes first.
    pub streams: Vec<StreamAnalysisViewModel>,
}

/// Session-scoped metadata (shared by all streams).
#[derive(Debug, Serialize)]
pub struct SessionInfoViewModel {
    pub session_id: String,
    pub provider: String,
    pub project_hash: String,
    pub project_root: Option<String>,
    pub model: Option<String>,
    pub log_files: Vec<String>,
    /// Spawn context when this entire session is a subagent session
    /// (e.g. Codex subagents stored in separate files).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spawned_by: Option<SpawnContextViewModel>,
}

/// Analysis of a single event stream (main conversation or a sidechain).
#[derive(Debug, Serialize)]
pub struct StreamAnalysisViewModel {
    pub stream_id: String,
    /// Where this stream was spawned from in the parent stream (sidechains only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spawned_by: Option<SpawnContextViewModel>,
    pub status: String,
    pub duration: Option<String>,
    pub start_time: Option<String>,
    pub context_summary: ContextWindowSummary,
    pub turns: Vec<TurnAnalysisViewModel>,
}

/// Spawn location (0-based indices) within the parent stream.
#[derive(Debug, Serialize)]
pub struct SpawnContextViewModel {
    pub turn_index: usize,
    pub step_index: usize,
}

#[derive(Debug, Serialize)]
pub struct ContextWindowSummary {
    pub current_tokens: u32,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct TurnAnalysisViewModel {
    pub turn_number: usize,
    pub timestamp: Option<String>,
    pub prev_tokens: u32,
    pub current_tokens: u32,
    pub context_usage: Option<ContextUsage>,
    pub is_heavy_load: bool,
    pub user_query: String,
    pub steps: Vec<AgentStepViewModel>,
    pub metrics: TurnMetrics,
    /// Child sessions (subagents) spawned from this turn
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub spawned_children: Vec<SpawnedChildViewModel>,
}

/// Information about a child session spawned from a turn
#[derive(Debug, Serialize)]
pub struct SpawnedChildViewModel {
    pub session_id: String,
    pub session_id_short: String,
}

#[derive(Debug, Serialize)]
pub struct ContextUsage {
    pub current_tokens: u32,
    pub max_tokens: u32,
    pub percentage: f64,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind")]
pub enum AgentStepViewModel {
    Thinking {
        duration: Option<String>,
        preview: String,
    },
    ToolCall {
        name: String,
        #[serde(skip)]
        arguments: agtrace_sdk::types::ToolCallPayload,
        #[serde(rename = "args")]
        args_formatted: Option<String>, // For JSON serialization compatibility
        result: String,
        is_error: bool,
        /// Agent ID if this tool spawned a subagent (e.g., Task tool)
        #[serde(skip_serializing_if = "Option::is_none")]
        agent_id: Option<String>,
    },
    ToolCallSequence {
        name: String,
        count: usize,
        #[serde(skip)]
        sample_arguments: agtrace_sdk::types::ToolCallPayload,
        #[serde(rename = "sample_args")]
        sample_args_formatted: Option<String>, // For JSON serialization compatibility
        has_errors: bool,
    },
    Message {
        text: String,
    },
    SystemEvent {
        description: String,
    },
}

#[derive(Debug, Serialize)]
pub struct TurnMetrics {
    pub total_delta: u32,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read_tokens: Option<i64>,
}

// Watch-related view models
#[derive(Debug, Clone, Serialize)]
pub struct ContextWindowUsageViewModel {
    pub fresh_input: i32,
    pub cache_creation: i32,
    pub cache_read: i32,
    pub output: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct StreamStateViewModel {
    pub session_id: String,
    pub project_root: Option<String>,
    pub start_time: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub model: Option<String>,
    pub context_window_limit: Option<u64>,
    pub current_usage: ContextWindowUsageViewModel,
    pub current_reasoning_tokens: i32,
    pub error_count: u32,
    pub event_count: usize,
    pub turn_count: usize,
    pub token_limit: Option<u64>,
    pub compaction_buffer_pct: Option<f64>,
}

/// Turn usage view model for TUI visualization
#[derive(Debug, Clone, Serialize)]
pub struct TurnUsageViewModel {
    pub turn_id: usize,
    pub title: String,
    pub prev_total: u32,
    pub delta: u32,
    pub is_heavy: bool,
    pub is_active: bool,
    pub recent_steps: Vec<StepItemViewModel>,
    pub start_time: Option<DateTime<Utc>>,
}

/// Step item for TUI turn history
#[derive(Debug, Clone, Serialize)]
pub struct StepItemViewModel {
    pub timestamp: DateTime<Utc>,
    pub emoji: String,
    pub description: String,
    pub token_usage: Option<u32>,
}

// --------------------------------------------------------
// CreateView Trait Implementations (Bridge to Views)
// --------------------------------------------------------

impl CreateView for SessionListViewModel {
    fn create_view<'a>(&'a self, mode: ViewMode) -> Box<dyn fmt::Display + 'a> {
        use crate::presentation::views::session::SessionListView;
        Box::new(SessionListView::new(self, mode))
    }
}

impl CreateView for SessionDetailViewModel {
    fn create_view<'a>(&'a self, mode: ViewMode) -> Box<dyn fmt::Display + 'a> {
        use crate::presentation::views::session::SessionDetailView;
        Box::new(SessionDetailView::new(self, mode))
    }
}

// --------------------------------------------------------
// Display Trait (for backward compatibility and default rendering)
// --------------------------------------------------------

impl fmt::Display for SessionListViewModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.create_view(ViewMode::default()))
    }
}

impl fmt::Display for SessionDetailViewModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.create_view(ViewMode::default()))
    }
}

impl fmt::Display for TurnAnalysisViewModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use crate::presentation::ViewMode;
        use crate::presentation::views::session::TurnView;
        write!(f, "{}", TurnView::new(self, ViewMode::Standard))
    }
}
