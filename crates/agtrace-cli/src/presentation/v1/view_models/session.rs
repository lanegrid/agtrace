use chrono::{DateTime, Utc};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct SessionViewModel {
    pub session_id: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub turns: Vec<TurnViewModel>,
    pub stats: SessionStatsViewModel,
}

#[derive(Debug, Clone)]
pub struct TurnViewModel {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub user_message: String,
    pub steps: Vec<StepViewModel>,
    pub stats: TurnStatsViewModel,
}

#[derive(Debug, Clone)]
pub struct StepViewModel {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub reasoning_text: Option<String>,
    pub message_text: Option<String>,
    pub tools: Vec<ToolExecutionViewModel>,
    pub usage: Option<TokenUsageViewModel>,
    pub is_failed: bool,
}

#[derive(Debug, Clone)]
pub struct ToolExecutionViewModel {
    pub name: String,
    pub arguments: Value,
    pub output: Option<String>,
    pub duration_ms: Option<i64>,
    pub is_error: bool,
}

#[derive(Debug, Clone)]
pub struct TokenUsageViewModel {
    pub input_tokens: i32,
    pub output_tokens: i32,
    pub total_tokens: i32,
    pub cache_creation_tokens: Option<i32>,
    pub cache_read_tokens: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct SessionStatsViewModel {
    pub total_turns: usize,
    pub total_steps: usize,
    pub total_tool_calls: usize,
}

#[derive(Debug, Clone)]
pub struct TurnStatsViewModel {
    pub total_steps: usize,
    pub total_tool_calls: usize,
}

#[derive(Debug, Clone)]
pub struct SessionDigestViewModel {
    pub session_id: String,
    pub source: String,
    pub opening: Option<String>,
    pub activation: Option<String>,
    pub tool_calls_total: usize,
    pub tool_failures_total: usize,
    pub max_e2e_ms: u64,
    pub max_tool_ms: u64,
    pub missing_tool_pairs: usize,
    pub loop_signals: usize,
    pub longest_chain: usize,
    pub recency_boost: u32,
    pub selection_reason: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionListEntryViewModel {
    pub id: String,
    pub provider: String,
    pub project_hash: String,
    pub start_ts: Option<String>,
    pub snippet: Option<String>,
}

// --------------------------------------------------------
// Facade: Re-export v2 types for backward compatibility
// --------------------------------------------------------

pub use crate::presentation::v2::view_models::ContextWindowUsageViewModel;
pub use crate::presentation::v2::view_models::ReactionViewModel;
pub use crate::presentation::v2::view_models::StepItemViewModel;
pub use crate::presentation::v2::view_models::StreamStateViewModel;
pub use crate::presentation::v2::view_models::TurnUsageViewModel;
