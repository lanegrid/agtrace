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
    /// Agent tree: this session's agent, its subagents / forks and child sessions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agents: Option<AgentNodeViewModel>,
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
}

/// One agent of the session's agent tree (index: log files + child sessions).
#[derive(Debug, Clone, Serialize)]
pub struct AgentNodeViewModel {
    pub agent_id: String,
    pub session_id: String,
    pub provider: String,
    /// `main`, `subagent`, `fork`, `teammate`, `codex_thread`.
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Provider call id of the spawning tool call in the parent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spawn_call_id: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<AgentNodeViewModel>,
}

/// Analysis of a single event stream (main conversation or a sidechain).
#[derive(Debug, Serialize)]
pub struct StreamAnalysisViewModel {
    pub stream_id: String,
    /// Agent id of the stream (`claude:<sid>`, `claude:<sid>/<aid>`, `codex:<thread>`).
    pub agent_id: String,
    /// Agent display name from the index (subagent description, teammate name).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub status: String,
    pub duration: Option<String>,
    pub start_time: Option<String>,
    pub context_summary: ContextWindowSummary,
    pub turns: Vec<TurnAnalysisViewModel>,
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
    /// Agents spawned by this turn's tool calls
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub spawned_children: Vec<SpawnedChildViewModel>,
}

/// An agent spawned by a tool call (matched through the index `spawn_call_id`).
#[derive(Debug, Clone, Serialize)]
pub struct SpawnedChildViewModel {
    pub agent_id: String,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
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
        /// Agent spawned by this call (Claude `Agent`, Codex `spawn_agent`)
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
