//! View models and UI state of the multi-agent watch TUI (design §6).
//!
//! `WatchScreenVm` is a read-only snapshot built by
//! [`crate::presentation::presenters::watch::build_screen`] from a
//! `WorkspaceView` plus the [`UiState`]; the views only lay it out.

use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use chrono::FixedOffset;
use serde::Serialize;

// ============================================================================
// UI state (owned by the handler, read by the presenter)
// ============================================================================

/// Pane that receives scroll keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Pane {
    #[default]
    Tree,
    Timeline,
    Feed,
}

impl Pane {
    pub fn next(self) -> Self {
        match self {
            Pane::Tree => Pane::Timeline,
            Pane::Timeline => Pane::Feed,
            Pane::Feed => Pane::Tree,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Pane::Tree => Pane::Feed,
            Pane::Timeline => Pane::Tree,
            Pane::Feed => Pane::Timeline,
        }
    }
}

/// Which feed entries are shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedFilter {
    #[default]
    All,
    /// Only entries sent / received / emitted by the selected agent.
    Selected,
}

/// Vertical position of a list whose newest rows are at the bottom.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Scroll {
    /// Stick to the newest rows.
    #[default]
    Follow,
    /// First visible row index (auto-follow off).
    Offset(usize),
}

impl Scroll {
    /// First visible row for `total` rows in a pane of `height` rows.
    pub fn start(self, total: usize, height: usize) -> usize {
        let max = total.saturating_sub(height);
        match self {
            Scroll::Follow => max,
            Scroll::Offset(o) => o.min(max),
        }
    }

    pub fn is_follow(self) -> bool {
        matches!(self, Scroll::Follow)
    }
}

/// Content heights (rows) of the scrollable panes in the last layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Viewport {
    pub tree: usize,
    pub timeline: usize,
    pub feed: usize,
}

/// How long a status-bar toast stays visible.
pub const TOAST_TTL: Duration = Duration::from_secs(2);

/// Short-lived status-bar message confirming a state change (set by the reducer).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toast {
    pub text: String,
    /// When the toast was raised.
    pub at: Instant,
}

impl Toast {
    pub fn new(text: impl Into<String>, at: Instant) -> Self {
        Self {
            text: text.into(),
            at,
        }
    }

    /// Still visible at `now`.
    pub fn is_live(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.at) < TOAST_TTL
    }
}

/// Mutable UI context of the watch TUI (selection, collapse, scroll, filters).
///
/// Selection is kept by agent id (not row index) so it survives tree changes.
#[derive(Debug, Clone)]
pub struct UiState {
    pub selected: Option<String>,
    pub collapsed: BTreeSet<String>,
    pub focus: Pane,
    pub timeline_scroll: Scroll,
    pub feed_scroll: Scroll,
    pub feed_filter: FeedFilter,
    /// Hide Done / Killed agents (kept when a descendant is still shown).
    pub hide_done: bool,
    /// Follow the most recently active agent.
    pub auto_select: bool,
    pub show_help: bool,
    /// Last state-change message; the presenter drops it once expired.
    pub toast: Option<Toast>,
    /// Scope description for the status bar ("project agtrace").
    pub scope: String,
    /// Offset used to print wall-clock times.
    pub utc_offset: FixedOffset,
    pub viewport: Viewport,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            selected: None,
            collapsed: BTreeSet::new(),
            focus: Pane::Tree,
            timeline_scroll: Scroll::Follow,
            feed_scroll: Scroll::Follow,
            feed_filter: FeedFilter::All,
            hide_done: false,
            auto_select: false,
            show_help: false,
            toast: None,
            scope: String::new(),
            utc_offset: FixedOffset::east_opt(0).expect("zero offset"),
            viewport: Viewport::default(),
        }
    }
}

impl UiState {
    pub fn new(scope: impl Into<String>, utc_offset: FixedOffset) -> Self {
        Self {
            scope: scope.into(),
            utc_offset,
            ..Self::default()
        }
    }
}

// ============================================================================
// Screen view model
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct WatchScreenVm {
    pub tree: Vec<AgentRowVm>,
    pub focus: FocusVm,
    pub feed: Vec<FeedRowVm>,
    pub status: StatusBarVm,
    pub focus_pane: Pane,
    /// Scroll positions echoed from the UI state (the views clamp them).
    pub timeline_scroll: Scroll,
    pub feed_scroll: Scroll,
    pub show_help: bool,
    /// Live toast text (status bar), if any.
    pub toast: Option<String>,
}

impl WatchScreenVm {
    /// Row index of the selected agent in `tree`.
    pub fn selected_index(&self) -> Option<usize> {
        self.tree.iter().position(|r| r.selected)
    }
}

/// Agent status as shown in the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusVm {
    Running,
    Idle,
    Done,
    Killed,
    Failed,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentRowVm {
    pub id: String,
    pub depth: u16,
    /// Per ancestor level below the roots (depth-1 entries): true when that
    /// ancestor has later siblings, i.e. a vertical guide continues.
    pub guides: Vec<bool>,
    pub is_last_sibling: bool,
    pub label: String,
    /// `claude_code` / `codex`.
    pub provider: String,
    /// `T` teammate, `S` subagent, `F` fork; none for main / Codex thread.
    pub badge: Option<char>,
    pub status: StatusVm,
    /// Context occupancy in percent; None when the window or usage is unknown.
    pub ctx_pct: Option<u16>,
    pub selected: bool,
    pub collapsed: bool,
    pub has_children: bool,
    /// Descendants not shown because this node is collapsed.
    pub hidden_descendants: usize,
    /// A tool call is currently open.
    pub busy_tool: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CtxVm {
    pub pct: u16,
    pub used_tokens: u64,
    pub window_tokens: u64,
    /// `log`, `1m`, `cache`, `table`, `cfg`, `obs`, `obs?`.
    pub provenance: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActivityVm {
    /// Tool call without a result yet.
    Tool {
        name: String,
        summary: String,
        elapsed_secs: i64,
        /// Other open tool calls.
        more: usize,
    },
    /// Nothing running; the last turn ended.
    TurnEnded { outcome: String, ago_secs: i64 },
}

#[derive(Debug, Clone, Serialize)]
pub struct FocusVm {
    /// None when the workspace has no agents yet.
    pub agent_id: Option<String>,
    pub title: String,
    pub provider: String,
    pub kind: String,
    pub team: Option<String>,
    pub status: StatusVm,
    pub model: Option<String>,
    pub ctx: Option<CtxVm>,
    pub activity: Option<ActivityVm>,
    pub rows: Vec<TimelineRowVm>,
    pub follow: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RowKind {
    User,
    Command,
    Assistant,
    Tool,
    ToolError,
    SubAction,
    MessageIn,
    MessageOut,
    Spawn,
    Lifecycle,
    Compaction,
    ModelChange,
    TurnEnd,
    Interrupt,
    Queued,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimelineRowVm {
    pub time: String,
    pub icon: &'static str,
    pub kind: RowKind,
    /// Short leading label (tool name, peer, `compact`, ...); may be empty.
    pub label: String,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedRowKind {
    Message,
    Spawn,
    Lifecycle,
}

#[derive(Debug, Clone, Serialize)]
pub struct FeedRowVm {
    pub time: String,
    pub kind: FeedRowKind,
    pub from: String,
    pub to: Vec<String>,
    /// `MESSAGE`, `NEW_TASK`, `FINAL_ANSWER`, `spawn`, `idle`, `done`, ...
    pub tag: String,
    /// Plaintext body / reason; None when encrypted or absent.
    pub text: Option<String>,
    pub encrypted: bool,
    /// Involves the selected agent (highlighted).
    pub involves_selected: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct StatusBarVm {
    pub scope: String,
    pub agents: usize,
    pub running: usize,
    pub idle: usize,
    /// Agents hidden by the "hide done" toggle.
    pub hidden: usize,
    /// Agents the "hide done" toggle hides when it is on.
    pub done_hideable: usize,
    /// Undecodable / schema-mismatched lines over all agents.
    pub diagnostics: u64,
    /// Watcher errors (I/O, permissions).
    pub errors: usize,
    pub last_error: Option<String>,
    pub feed_filter: FeedFilter,
    pub hide_done: bool,
    pub auto_select: bool,
    /// Collapsed tree nodes.
    pub collapsed: usize,
}

// ============================================================================
// Console (line printer) view model
// ============================================================================

/// Snapshot for `watch --mode console`: the screen (tree, feed, status) plus the
/// timeline of every agent, with stable keys so each row is printed once.
#[derive(Debug, Clone, Serialize)]
pub struct ConsoleVm {
    pub screen: WatchScreenVm,
    /// Per agent, in tree order.
    pub timelines: Vec<AgentTimelineVm>,
    /// Event id of each `screen.feed` row (same order).
    pub feed_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentTimelineVm {
    pub agent_id: String,
    pub label: String,
    pub rows: Vec<KeyedRow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct KeyedRow {
    /// Event id of the row.
    pub key: String,
    pub row: TimelineRowVm,
}
