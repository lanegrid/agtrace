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

/// Top-level screen of the watch TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Screen {
    /// `1`: every agent with status, context, activity lane and current work.
    #[default]
    Overview,
    /// `2`: agent tree + selected agent's timeline + message feed.
    Agents,
    /// `Enter` on an agent: instructions, current work, result and timeline.
    Detail,
}

/// Time span of the overview's activity lanes (`+` / `-`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LaneWindow {
    M15,
    #[default]
    M60,
    H4,
    /// Since the oldest shown agent started.
    All,
}

impl LaneWindow {
    /// Next wider window (saturates at `All`).
    pub fn wider(self) -> Self {
        match self {
            LaneWindow::M15 => LaneWindow::M60,
            LaneWindow::M60 => LaneWindow::H4,
            LaneWindow::H4 | LaneWindow::All => LaneWindow::All,
        }
    }

    /// Next narrower window (saturates at 15m).
    pub fn narrower(self) -> Self {
        match self {
            LaneWindow::All => LaneWindow::H4,
            LaneWindow::H4 => LaneWindow::M60,
            LaneWindow::M60 | LaneWindow::M15 => LaneWindow::M15,
        }
    }

    /// Length in seconds; None for `All`.
    pub fn secs(self) -> Option<i64> {
        match self {
            LaneWindow::M15 => Some(15 * 60),
            LaneWindow::M60 => Some(60 * 60),
            LaneWindow::H4 => Some(4 * 3600),
            LaneWindow::All => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            LaneWindow::M15 => "15m",
            LaneWindow::M60 => "60m",
            LaneWindow::H4 => "4h",
            LaneWindow::All => "all",
        }
    }
}

/// Section of the agent detail screen (Tab cycles, j/k scroll the focused one).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DetailSection {
    #[default]
    Instructions,
    Now,
    Result,
    Timeline,
}

impl DetailSection {
    pub const ALL: [DetailSection; 4] = [
        DetailSection::Instructions,
        DetailSection::Now,
        DetailSection::Result,
        DetailSection::Timeline,
    ];

    pub fn index(self) -> usize {
        match self {
            DetailSection::Instructions => 0,
            DetailSection::Now => 1,
            DetailSection::Result => 2,
            DetailSection::Timeline => 3,
        }
    }

    pub fn next(self) -> Self {
        Self::ALL[(self.index() + 1) % 4]
    }

    pub fn prev(self) -> Self {
        Self::ALL[(self.index() + 3) % 4]
    }

    pub fn title(self) -> &'static str {
        match self {
            DetailSection::Instructions => "Instructions",
            DetailSection::Now => "Now",
            DetailSection::Result => "Result",
            DetailSection::Timeline => "Timeline",
        }
    }

    /// Initial scroll: text sections start at the top, the timeline follows.
    pub fn initial_scroll(self) -> Scroll {
        match self {
            DetailSection::Timeline => Scroll::Follow,
            _ => Scroll::Offset(0),
        }
    }
}

/// Initial scroll of every detail section.
pub fn initial_detail_scroll() -> [Scroll; 4] {
    DetailSection::ALL.map(DetailSection::initial_scroll)
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

/// Wrapped line count and visible height of one detail section (last frame).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SectionMetrics {
    pub total: usize,
    pub height: usize,
}

/// Content heights (rows) of the scrollable panes in the last layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Viewport {
    pub tree: usize,
    pub timeline: usize,
    pub feed: usize,
    /// Cells of an overview activity lane (0 = no room for lanes).
    pub lane_cols: usize,
    /// Detail sections in [`DetailSection::index`] order.
    pub detail: [SectionMetrics; 4],
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
    pub screen: Screen,
    /// Screen `Esc` returns to from the detail screen.
    pub detail_return: Screen,
    /// Agent shown on the detail screen (fixed while it is open).
    pub detail_agent: Option<String>,
    pub detail_section: DetailSection,
    /// Per detail section, in [`DetailSection::index`] order.
    pub detail_scroll: [Scroll; 4],
    /// Overview activity lane span.
    pub window: LaneWindow,
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
            screen: Screen::Overview,
            detail_return: Screen::Overview,
            detail_agent: None,
            detail_section: DetailSection::Instructions,
            detail_scroll: initial_detail_scroll(),
            window: LaneWindow::M60,
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
    pub screen: Screen,
    /// Built on the overview screen only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overview: Option<OverviewVm>,
    /// Built on the detail screen only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<DetailVm>,
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
// Overview screen
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct OverviewVm {
    /// `60m`, `4h`, `all`.
    pub window: String,
    /// Time covered by one lane cell (`2m`).
    pub cell: String,
    /// One row per tree row (same order, fold and hide-done state).
    pub rows: Vec<OverviewRowVm>,
}

/// Summary line printed above each root's block.
#[derive(Debug, Clone, Serialize)]
pub struct RootHeaderVm {
    pub label: String,
    pub provider: String,
    pub status: StatusVm,
    /// Since the root started.
    pub age_secs: Option<i64>,
    pub model: Option<String>,
    pub ctx: Option<CtxVm>,
    pub compactions: u32,
    /// Agents in the root's tree (itself included) and how many are running.
    pub agents: usize,
    pub running: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct OverviewRowVm {
    pub id: String,
    pub depth: u16,
    pub guides: Vec<bool>,
    pub is_last_sibling: bool,
    pub label: String,
    pub badge: Option<char>,
    pub provider: String,
    pub status: StatusVm,
    pub ctx: Option<CtxVm>,
    /// One glyph per cell, oldest first: `▁▂▃▅▆` by event density, `⟲` compaction,
    /// `·` running without events, blank when idle / not existing.
    pub lane: String,
    /// One tone per lane cell: `r` running, `i` idle, `d` ended, `c` compaction,
    /// ` ` none.
    pub lane_tones: String,
    pub now: NowVm,
    pub selected: bool,
    pub collapsed: bool,
    pub hidden_descendants: usize,
    /// Present on root rows.
    pub root: Option<RootHeaderVm>,
}

/// What the agent is doing now, in one cell.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NowVm {
    Tool {
        name: String,
        summary: String,
        elapsed_secs: i64,
    },
    /// Running without an open tool: the latest assistant text / reasoning.
    Said {
        text: String,
    },
    Idle {
        secs: i64,
    },
    /// Ended with a result (excerpt).
    Result {
        text: String,
    },
    /// Ended (done / killed / failed) `secs` ago, with the reason when known.
    Ended {
        status: StatusVm,
        secs: Option<i64>,
        reason: Option<String>,
    },
    None,
}

// ============================================================================
// Detail screen
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct DetailVm {
    pub agent_id: String,
    pub title: String,
    /// `teammate of lead (team t)`, `subagent of lead`, `thread of /root`, `main session`.
    pub relation: String,
    pub agent_type: Option<String>,
    pub model: Option<String>,
    pub status: StatusVm,
    /// Time in the current status.
    pub status_secs: Option<i64>,
    pub ctx: Option<CtxVm>,
    /// Context history (`▁`..`█` occupancy, `⟲` compaction), oldest first.
    pub spark: String,
    pub totals: TotalsVm,
    pub instructions: Vec<InstructionVm>,
    pub now: DetailNowVm,
    pub result: ResultVm,
    pub timeline: Vec<TimelineRowVm>,
    pub section: DetailSection,
    pub scroll: [Scroll; 4],
}

#[derive(Debug, Clone, Serialize)]
pub struct TotalsVm {
    pub input_tokens: u64,
    pub output_tokens: u64,
    /// Some output counts are stream-start snapshots: `output_tokens` is a lower bound.
    pub output_partial: bool,
    pub turns: u32,
    pub tool_calls: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct InstructionVm {
    pub time: String,
    /// `spawned by lead · NEW_TASK`, `lead → v8fix1 MESSAGE`, `user`, `queued`.
    pub header: String,
    /// Full text; None when encrypted or absent.
    pub text: Option<String>,
    pub encrypted: bool,
    /// Extra facts (Codex encrypted task: sender, path, requested model).
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DetailNowVm {
    pub status: StatusVm,
    pub tool: Option<ActivityVm>,
    /// Latest assistant text (or reasoning when there is no text), in full.
    pub said: Option<String>,
    /// `said` is reasoning, not assistant text.
    pub said_is_reasoning: bool,
    pub said_time: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResultVm {
    /// Still working (no result yet).
    Pending { status: StatusVm },
    Reported {
        time: String,
        /// `FINAL_ANSWER`, `HANDBACK`, `TASK_NOTIFY`.
        tag: String,
        text: Option<String>,
        encrypted: bool,
    },
    /// Ended without a reported result: its last assistant message.
    LastMessage {
        time: String,
        text: String,
        status: StatusVm,
        reason: Option<String>,
    },
    Ended {
        status: StatusVm,
        reason: Option<String>,
    },
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
