//! TUI-specific ViewModels for Watch command
//!
//! These ViewModels define the complete data contract for the TUI Renderer.
//! They contain ONLY primitive types and computed values - NO domain logic.
//! The TUI Renderer should be able to draw the screen using ONLY this data.
//!
//! ## Multi-Page Architecture
//!
//! This ViewModel is organized hierarchically to support multiple pages/tabs:
//! - Common components (status_bar) are always present
//! - Page-specific components (dashboard, timeline, turn_history) belong to specific tabs
//! - Future pages (e.g., turn_details) can be added as Option<T> fields
//!
//! The Presenter decides which components to populate based on the active tab.
//! The Renderer uses the active_tab to determine which components to render.

use chrono::{DateTime, Utc};
use serde::Serialize;

use super::common::StatusLevel;

/// Complete screen state for TUI rendering
///
/// Currently contains all data for the Dashboard page.
/// Future pages can be added as optional fields (e.g., turn_details: Option<TurnDetailsViewModel>).
#[derive(Debug, Clone, Serialize)]
pub struct TuiScreenViewModel {
    /// Dashboard component (session overview) - Dashboard page
    pub dashboard: DashboardViewModel,
    /// Timeline component (event stream) - Dashboard page
    pub timeline: TimelineViewModel,
    /// Turn history component (turn list) - Dashboard page
    pub turn_history: TurnHistoryViewModel,
    /// Status bar component (always visible on all pages)
    pub status_bar: StatusBarViewModel,
}

/// Dashboard component (top section with session overview)
#[derive(Debug, Clone, Serialize)]
pub struct DashboardViewModel {
    pub title: String,
    pub sub_title: Option<String>,
    pub session_id: String,
    pub project_root: Option<String>,
    pub log_path: Option<String>,
    pub model: Option<String>,
    pub start_time: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub elapsed_seconds: u64,

    // Context window usage (raw data for JSON API)
    pub context_total: u64,             // Total tokens used
    pub context_limit: Option<u64>,     // Context window limit (None if unknown)
    pub context_usage_pct: Option<f64>, // 0.0 - 1.0 (None if limit unknown)
    pub context_color: StatusLevel,     // Color decision already made
    pub context_breakdown: ContextBreakdownViewModel,
}

/// Context window usage breakdown
#[derive(Debug, Clone, Serialize)]
pub struct ContextBreakdownViewModel {
    pub fresh_input: u64,
    pub cache_creation: u64,
    pub cache_read: u64,
    pub output: u64,
    pub total: u64,
}

/// Timeline component (recent events stream)
#[derive(Debug, Clone, Serialize)]
pub struct TimelineViewModel {
    pub events: Vec<TimelineEventViewModel>,
    pub total_count: usize,
    pub displayed_count: usize,
}

/// Single timeline event item
#[derive(Debug, Clone, Serialize)]
pub struct TimelineEventViewModel {
    pub timestamp: DateTime<Utc>,
    pub relative_time: String, // e.g., "2s ago" (pre-formatted)
    pub icon: String,          // Emoji or symbol
    pub description: String,   // Short summary (pre-formatted, truncated)
    pub level: StatusLevel,    // For coloring
}

/// Turn history component (left sidebar with turn list)
#[derive(Debug, Clone, Serialize)]
pub struct TurnHistoryViewModel {
    pub turns: Vec<TurnItemViewModel>,
    pub active_turn_index: Option<usize>,
    pub waiting_state: Option<WaitingState>,
}

/// Waiting state information with contextual hints
#[derive(Debug, Clone, Serialize)]
pub struct WaitingState {
    pub kind: WaitingKind,
    pub session_id: Option<String>,
    pub project_root: Option<String>,
    pub event_count: Option<usize>,
    pub last_activity_relative: Option<String>,
}

/// Type of waiting state
#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum WaitingKind {
    NoSession,      // Waiting for agent to start
    Analyzing,      // Session detected but not assembled yet
    MissingContext, // Max context unknown (rare)
}

/// Single turn item in history
#[derive(Debug, Clone, Serialize)]
pub struct TurnItemViewModel {
    pub turn_id: usize,
    pub title: String,                 // Truncated user message
    pub slash_command: Option<String>, // Slash command name (e.g., "/skaffold-repo")
    pub is_active: bool,
    pub is_heavy: bool, // Indicates if this turn consumed significant tokens
    pub context_compacted: bool, // True if context was reset during this turn

    // Stacked bar visualization (pre-computed) - v1-style cumulative display
    pub prev_total: u32,     // Total tokens before this turn (0 if compacted)
    pub delta_tokens: u32,   // Tokens added by this turn (or new baseline if compacted)
    pub usage_ratio: f64,    // Total usage ratio after this turn (0.0 - 1.0)
    pub prev_ratio: f64,     // Usage ratio before this turn (0.0 - 1.0)
    pub delta_ratio: f64,    // Delta ratio for this turn (0.0 - 1.0)
    pub bar_width: u16,      // Total bar width in characters
    pub prev_bar_width: u16, // Previous bar width in characters
    pub delta_color: StatusLevel,

    // Step preview (for active turn)
    pub recent_steps: Vec<StepPreviewViewModel>,
    pub start_time: Option<DateTime<Utc>>,

    // Child streams spawned during this turn (displayed indented below)
    pub child_streams: Vec<ChildStreamViewModel>,
}

/// Child stream (sidechain/subagent) spawned from a parent turn
#[derive(Debug, Clone, Serialize)]
pub struct ChildStreamViewModel {
    /// Stream identifier (e.g., "sidechain:abc123", "subagent:review")
    pub stream_label: String,
    /// First user message (truncated)
    pub first_message: String,
    /// Last turn's context bar data (only last turn shown, others hidden)
    pub last_turn: Option<Box<TurnItemViewModel>>,
    /// Whether this child stream is currently active
    pub is_active: bool,
}

/// Step preview for active turn
#[derive(Debug, Clone, Serialize)]
pub struct StepPreviewViewModel {
    pub timestamp: DateTime<Utc>,
    pub icon: String,        // Pre-determined emoji
    pub description: String, // Pre-formatted, truncated
    pub token_usage: Option<u32>,
}

/// Status bar component (bottom bar)
#[derive(Debug, Clone, Serialize)]
pub struct StatusBarViewModel {
    pub event_count: usize,
    pub turn_count: usize,
    pub status_message: String, // e.g., "Watching session abc123..."
    pub status_level: StatusLevel,
}
