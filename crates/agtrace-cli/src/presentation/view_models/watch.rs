//! View models and UI state of the multi-agent watch TUI (design §6).
//!
//! The TUI is built around an always-visible **navigator** (scope → sessions →
//! agent trees, with folded groups) and a content pane that shows the selected
//! node. `WatchScreenVm` is a read-only snapshot built by
//! [`crate::presentation::presenters::watch::build_screen`] from a
//! `WorkspaceView` plus the [`UiState`]; the views only lay it out.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use chrono::FixedOffset;
use serde::Serialize;

// ============================================================================
// UI state (owned by the handler, read by the presenter)
// ============================================================================

/// Pane that receives the movement / scroll keys (`Tab` cycles).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Pane {
    /// The navigator: ↑/↓ move the selection, ←/→ walk the hierarchy.
    #[default]
    Navigator,
    /// The content pane of the selected node: ↑/↓ scroll it.
    Content,
    /// The message feed: ↑/↓ scroll it.
    Feed,
}

impl Pane {
    pub fn next(self) -> Self {
        match self {
            Pane::Navigator => Pane::Content,
            Pane::Content => Pane::Feed,
            Pane::Feed => Pane::Navigator,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Pane::Navigator => Pane::Feed,
            Pane::Content => Pane::Navigator,
            Pane::Feed => Pane::Content,
        }
    }
}

/// Navigator key of the top (scope) node.
pub const NAV_TOP: &str = "@top";
/// Navigator key of the group of older sessions.
pub const NAV_OLDER: &str = "@older";

/// Navigator key of the group of finished children folded under `parent`.
pub fn fold_key(parent: &str) -> String {
    format!("@fold:{parent}")
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

/// Section of the agent detail (`i` `n` `r` `t` pick one, j/k scroll it).
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

    /// Key that focuses the section (`i` `n` `r` `t`).
    pub fn key(self) -> char {
        match self {
            DetailSection::Instructions => 'i',
            DetailSection::Now => 'n',
            DetailSection::Result => 'r',
            DetailSection::Timeline => 't',
        }
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

/// A parent's finished children fold into one navigator group only when there
/// are at least this many of them (a single one is shown as it is).
pub const NAV_FOLD_MIN: usize = 2;

/// Navigator width `clamp(24, 30%, 40)` columns; below this terminal width `s`
/// can hide it.
pub const NARROW_WIDTH: u16 = 80;

/// Wrapped line count and visible height of one scrollable text (last frame).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SectionMetrics {
    pub total: usize,
    pub height: usize,
}

/// Sizes of the panes in the last layout (the reducer clamps scrolls and pages
/// against them).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Viewport {
    /// Navigator rows (0 when hidden).
    pub nav: usize,
    pub feed: usize,
    /// Cells of an overview activity lane (0 = no room for lanes).
    pub lane_cols: usize,
    /// The terminal is narrower than [`NARROW_WIDTH`] (`s` may hide the navigator).
    pub narrow: bool,
    /// Overview-like content (top, session, folded group, older sessions).
    pub content: SectionMetrics,
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

/// Mutable UI context of the watch TUI (selection, expansion, scroll, filters).
///
/// The selection is a navigator key (an agent id, [`NAV_TOP`], [`NAV_OLDER`] or a
/// [`fold_key`]), not a row index, so it survives tree changes.
#[derive(Debug, Clone)]
pub struct UiState {
    /// Selected navigator node; None = the top node.
    pub selected: Option<String>,
    /// Expand (true) / collapse (false) overrides of navigator nodes; the others
    /// use their default (sessions open when there is only one, agents open,
    /// folded groups and older sessions closed).
    pub open: BTreeMap<String, bool>,
    pub focus: Pane,
    /// On a session node, show its root agent's detail instead of the session
    /// overview (`i` `n` `r` `t`, or → on a session without children).
    pub root_detail: bool,
    /// Section in effect in the agent detail.
    pub detail_section: DetailSection,
    /// Section the user last picked (`i` `n` `r` `t`); kept across agents.
    pub detail_pref: Option<DetailSection>,
    /// A new agent was selected: the presenter picks the section
    /// ([`DetailVm::pick_section`]) and the handler stores it back.
    pub detail_auto: bool,
    /// Per detail section, in [`DetailSection::index`] order.
    pub detail_scroll: [Scroll; 4],
    /// First visible line of overview-like content.
    pub content_scroll: usize,
    pub feed_scroll: Scroll,
    /// Overview activity lane span.
    pub window: LaneWindow,
    /// Show finished (done / killed) agents in place. Off (default): a parent's
    /// finished children fold into one navigator group (when there are at least
    /// [`NAV_FOLD_MIN`] of them).
    pub show_done: bool,
    /// `/` name filter (case-insensitive substring); empty = off. Matching agents
    /// are shown with their ancestors, ignoring folds.
    pub filter: String,
    /// The filter is being typed (keys go to the filter text).
    pub filter_editing: bool,
    /// Follow the most recently active agent shown in the navigator.
    pub auto_select: bool,
    /// Navigator hidden (`s`, only below [`NARROW_WIDTH`] columns).
    pub nav_hidden: bool,
    pub show_help: bool,
    /// Last state-change message; the presenter drops it once expired.
    pub toast: Option<Toast>,
    /// Scope description ("project agtrace · since 2h").
    pub scope: String,
    /// Offset used to print wall-clock times.
    pub utc_offset: FixedOffset,
    pub viewport: Viewport,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            selected: None,
            open: BTreeMap::new(),
            focus: Pane::Navigator,
            root_detail: false,
            detail_section: DetailSection::Instructions,
            detail_pref: None,
            detail_auto: true,
            detail_scroll: initial_detail_scroll(),
            content_scroll: 0,
            feed_scroll: Scroll::Follow,
            window: LaneWindow::M60,
            show_done: false,
            filter: String::new(),
            filter_editing: false,
            auto_select: false,
            nav_hidden: false,
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
    pub nav: NavVm,
    /// What the content pane shows (by the selected node).
    pub content: ContentVm,
    /// Every session of the workspace (overview summary, older sessions list).
    pub sessions: SessionsVm,
    /// Top, session and folded-group content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overview: Option<OverviewVm>,
    /// Agent content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<DetailVm>,
    /// Messages scoped to the selection.
    pub feed: Vec<FeedRowVm>,
    /// `all`, `this session`, the agent's label, ...
    pub feed_scope: String,
    pub status: StatusBarVm,
    pub focus_pane: Pane,
    /// Scroll positions echoed from the UI state (the views clamp them).
    pub content_scroll: usize,
    pub feed_scroll: Scroll,
    pub nav_hidden: bool,
    pub show_help: bool,
    /// Live toast text (status bar), if any.
    pub toast: Option<String>,
}

impl WatchScreenVm {
    /// Row index of the selected navigator node.
    pub fn selected_index(&self) -> Option<usize> {
        self.nav.rows.iter().position(|r| r.selected)
    }

    pub fn selected_row(&self) -> Option<&NavRowVm> {
        self.nav.rows.iter().find(|r| r.selected)
    }
}

// ============================================================================
// Navigator
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct NavVm {
    /// Rows in display order: the top node, then the sessions and their trees.
    pub rows: Vec<NavRowVm>,
}

/// Kind of a navigator node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NavKind {
    /// The watch scope (project, all projects, session).
    Top,
    /// One session: its root agent and tree.
    Session,
    /// An agent below a session root.
    Agent,
    /// A parent's finished (done / killed) children and earlier transcripts.
    Fold,
    /// Sessions that ended more than an hour ago.
    Older,
}

#[derive(Debug, Clone, Serialize)]
pub struct NavRowVm {
    /// Selection key (agent id, [`NAV_TOP`], [`NAV_OLDER`], [`fold_key`]).
    pub key: String,
    pub kind: NavKind,
    /// 0 top, 1 sessions (and the older group), 2+ below.
    pub depth: u16,
    /// Per ancestor level below the sessions: true when that ancestor has later
    /// siblings (a vertical guide continues).
    pub guides: Vec<bool>,
    pub is_last_sibling: bool,
    /// Key of the parent node (None for the top node).
    pub parent: Option<String>,
    pub label: String,
    /// `claude_code` / `codex`.
    pub provider: String,
    /// `T` teammate, `S` subagent, `F` fork.
    pub badge: Option<char>,
    /// Agent status; a session's liveness (busy → running, idle, ended → done).
    pub status: StatusVm,
    /// Session nodes: liveness and background flag.
    pub state: Option<SessionStateVm>,
    pub bg: bool,
    /// Top node: live sessions in scope.
    pub live: usize,
    /// Folded group counts.
    pub folded: Option<FoldedVm>,
    pub ctx_pct: Option<u16>,
    /// Has children (possibly hidden while collapsed).
    pub expandable: bool,
    pub expanded: bool,
    /// Folded group: its first item; older group: the first older session.
    pub first_item: Option<String>,
    /// Matches the `/` filter.
    pub matched: bool,
    pub selected: bool,
}

/// What the content pane shows for the selected node.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentVm {
    /// Top node: the multi-session overview.
    Overview,
    /// Session node: that session's overview.
    Session { id: String, has_transcript: bool },
    /// Agent node (or a session's root agent): the agent detail.
    Agent { id: String },
    /// Folded group: its items as overview rows.
    Fold { parent: String, folded: FoldedVm },
    /// The older sessions, as session lines.
    Older { count: usize },
}

/// Agent status as shown in the navigator.
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

/// One agent of the console tree (`watch --mode console`).
#[derive(Debug, Clone, Serialize)]
pub struct AgentRowVm {
    pub id: String,
    pub depth: u16,
    pub label: String,
    /// `claude_code` / `codex`.
    pub provider: String,
    /// `T` teammate, `S` subagent, `F` fork; none for main / Codex thread.
    pub badge: Option<char>,
    pub status: StatusVm,
    /// Context occupancy in percent; None when the window or usage is unknown.
    pub ctx_pct: Option<u16>,
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
}

#[derive(Debug, Clone, Serialize)]
pub struct StatusBarVm {
    /// Breadcrumb of the selection: scope, session, agent / group.
    pub crumbs: Vec<String>,
    /// Sessions of the workspace and how many are live.
    pub sessions: usize,
    pub live: usize,
    /// Agents in scope.
    pub agents: usize,
    pub running: usize,
    pub idle: usize,
    /// Finished agents folded into navigator groups (`d` shows them).
    pub folded: usize,
    /// Undecodable / schema-mismatched lines over all agents.
    pub diagnostics: u64,
    /// Watcher errors (I/O, permissions).
    pub errors: usize,
    pub last_error: Option<String>,
    pub show_done: bool,
    pub auto_select: bool,
    /// `/` filter text (empty = off) and whether it is being typed.
    pub filter: String,
    pub filter_editing: bool,
    /// Navigator nodes matching the filter.
    pub matches: usize,
}

// ============================================================================
// Overview content
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct OverviewVm {
    /// `60m`, `4h`, `all`.
    pub window: String,
    /// Time covered by one lane cell (`2m`).
    pub cell: String,
    /// One row per shown agent, in tree order.
    pub rows: Vec<OverviewRowVm>,
    /// Older sessions left out (listed under the navigator's older group).
    pub older_hidden: usize,
}

/// Summary line printed above each root's block (one per session).
#[derive(Debug, Clone, Serialize)]
pub struct RootHeaderVm {
    /// Session display name.
    pub label: String,
    pub provider: String,
    pub bg: bool,
    pub state: SessionStateVm,
    pub status: StatusVm,
    /// Since the root started.
    pub age_secs: Option<i64>,
    pub model: Option<String>,
    /// Reasoning effort (`high`).
    pub effort: Option<String>,
    pub ctx: Option<CtxVm>,
    pub compactions: u32,
    /// Agents in the root's tree (itself included) and how many are running.
    pub agents: usize,
    pub running: usize,
    /// Finished agents of the session folded away (printed after its rows).
    pub folded: FoldedVm,
}

/// Finished agents (and the session's other transcripts) folded into one line.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct FoldedVm {
    pub done: usize,
    pub killed: usize,
    /// Earlier / `/clear` transcripts of the session.
    pub transcripts: usize,
}

impl FoldedVm {
    pub fn total(&self) -> usize {
        self.done + self.killed + self.transcripts
    }
}

// ============================================================================
// Sessions
// ============================================================================

/// Liveness of a session (sort order).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStateVm {
    Busy,
    Idle,
    Recent,
    Older,
}

impl SessionStateVm {
    pub fn is_live(self) -> bool {
        matches!(self, SessionStateVm::Busy | SessionStateVm::Idle)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionsVm {
    /// Watch scope (`project agtrace · since 2h`).
    pub scope: String,
    /// Every session, in order (live, recent, older).
    pub rows: Vec<SessionRowVm>,
    pub live: usize,
    pub recent: usize,
    pub older: usize,
}

impl SessionsVm {
    pub fn total(&self) -> usize {
        self.live + self.recent + self.older
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionRowVm {
    /// Root agent id.
    pub id: String,
    /// `claude_code` / `codex`.
    pub provider: String,
    pub name: String,
    /// The name is only the short id.
    pub name_is_id: bool,
    pub short_id: String,
    pub bg: bool,
    pub state: SessionStateVm,
    /// False: a live process that has not written a transcript yet.
    pub has_transcript: bool,
    pub agents: usize,
    pub running: usize,
    /// Root context occupancy.
    pub ctx: Option<CtxVm>,
    /// Since the last write.
    pub last_secs: Option<i64>,
    /// What the root is doing now.
    pub now: NowVm,
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
    /// Present on root rows (the top overview's session headers).
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
    /// Running between tools with a task in progress: its active form.
    Task {
        text: String,
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
// Agent detail
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct DetailVm {
    pub agent_id: String,
    pub title: String,
    /// `teammate of lead (team t)`, `subagent of lead`, `thread of /root`, `main session`.
    pub relation: String,
    pub agent_type: Option<String>,
    pub model: Option<String>,
    /// Reasoning effort (`high`): own log, else requested by the spawn call.
    pub effort: Option<String>,
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

impl DetailVm {
    /// Section with nothing to show for this agent.
    pub fn is_empty(&self, s: DetailSection) -> bool {
        match s {
            DetailSection::Instructions => self.instructions.is_empty(),
            DetailSection::Now => {
                self.now.tool.is_none() && self.now.plan.is_empty() && self.now.said.is_none()
            }
            DetailSection::Result => !self.has_result(),
            DetailSection::Timeline => self.timeline.is_empty(),
        }
    }

    /// A reported result or a last message stands in for one.
    pub fn has_result(&self) -> bool {
        matches!(
            self.result,
            ResultVm::Reported { .. } | ResultVm::LastMessage { .. }
        )
    }

    /// Section shown when the detail opens: the remembered choice when this agent
    /// has something there, else Now while running, the result once ended with
    /// one, else the timeline; never an empty section when another has content.
    pub fn pick_section(&self, pref: Option<DetailSection>) -> DetailSection {
        if let Some(p) = pref.filter(|p| !self.is_empty(*p)) {
            return p;
        }
        let ended = matches!(
            self.status,
            StatusVm::Done | StatusVm::Killed | StatusVm::Failed
        );
        let first = match self.status {
            StatusVm::Running => DetailSection::Now,
            _ if ended && self.has_result() => DetailSection::Result,
            _ => DetailSection::Timeline,
        };
        [
            first,
            DetailSection::Timeline,
            DetailSection::Now,
            DetailSection::Instructions,
            DetailSection::Result,
        ]
        .into_iter()
        .find(|s| !self.is_empty(*s))
        .unwrap_or(DetailSection::Timeline)
    }
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
    /// What the agent is trying to do (goal, task list, plan text).
    pub plan: PlanVm,
    /// Latest assistant text (or reasoning when there is no text), in full.
    pub said: Option<String>,
    /// `said` is reasoning, not assistant text.
    pub said_is_reasoning: bool,
    pub said_time: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct PlanVm {
    /// `objective` and its status (`active`, `paused`, ...).
    pub goal: Option<(String, Option<String>)>,
    pub tasks: Vec<TaskVm>,
    /// Latest plan text (Codex plan mode) and its time.
    pub text: Option<String>,
    pub text_time: Option<String>,
}

impl PlanVm {
    pub fn is_empty(&self) -> bool {
        self.goal.is_none() && self.tasks.is_empty() && self.text.is_none()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatusVm {
    Pending,
    InProgress,
    Completed,
    Other,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskVm {
    pub status: TaskStatusVm,
    /// Active form while in progress, else the subject (`#id` when unknown).
    pub text: String,
    /// Another agent that created / last updated the task.
    pub by: Option<String>,
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

/// Snapshot for `watch --mode console`: the agent tree of every session (nothing
/// folded), the feed, plus the timeline of every agent, with stable keys so each
/// row is printed once.
#[derive(Debug, Clone, Serialize)]
pub struct ConsoleVm {
    pub tree: Vec<AgentRowVm>,
    pub feed: Vec<FeedRowVm>,
    /// Per agent, in tree order.
    pub timelines: Vec<AgentTimelineVm>,
    /// Event id of each `feed` row (same order).
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
