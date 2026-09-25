//! Agent graph and live per-agent state (design §4.3): a pure fold of
//! [`WorkspaceEvent`]s into a [`WorkspaceView`] (agent tree, statuses, feed).
//!
//! The runtime watcher produces `WorkspaceEvent`s; the SDK folds them with
//! [`WorkspaceView::apply`] and calls [`WorkspaceView::tick`] periodically so that
//! time-based (staleness) rules fire without new input. Nothing here does I/O.
//!
//! - **Handle resolution**: `AgentHandle`s found in one agent's log (`Id`,
//!   `NativeAgentId`, `TeamMember`, `Path`) are resolved against the known agents.
//!   Unresolvable handles are kept as pending links and retried whenever the agent
//!   set changes (late discovery).
//! - **Linking**: an `AgentSpawn` in P resolving to C sets `C.parent = P`; a teammate
//!   without a known spawn is linked to the lead its team config names (the shared
//!   rule in [`teammate_parent`], also used by the index agent tree). Unlinked children are shown
//!   under their root.
//! - **Status**: see [`status`] for the priority table.
//! - **Feed**: inter-agent messages (the sender's and the recipient's copy are
//!   paired one-to-one into a single entry, also when a handle resolves late),
//!   spawns and lifecycle changes (repeated reports of one transition merged).

mod context_seam;
pub mod detail;
mod feed;
mod input;
mod parent;
mod ring;
mod session;
pub mod status;
mod timeline;
mod view;

pub use context_seam::{CatalogResolver, ContextEvidence, ContextWindow, NoWindow, WindowResolver};
pub use detail::{
    ActivityBucket, ActivityHistory, AgentDetail, AgentResult, ContextPoint, DETAIL_TEXT_MAX,
    Instruction, InstructionKind, PLAN_TASK_CAPACITY, PlanGoal, PlanState, PlanTask, Said,
    StatusHistory, StatusPoint, TaskChange, UsageTotals,
};
pub use feed::{
    FeedEntry, FeedKind, FeedParty, LIFECYCLE_DEDUPE_WINDOW, MESSAGE_CLOCK_SKEW,
    MESSAGE_DELIVERY_WINDOW,
};
pub use input::{ProcessStatus, SideStateUpdate, TeamMember, WorkspaceEvent};
pub use parent::{
    RuntimeAliases, TeammateSpawn, find_teammate_spawn, team_lead_agent, teammate_parent,
    teammate_spawns,
};
pub use ring::RingBuffer;
pub use session::{
    CODEX_SESSION_LIVE, SESSION_RECENT, Session, SessionFold, SessionNameSource, SessionState,
};
pub use status::{AgentStatus, StatusSource};
pub use timeline::{
    RunningTool, TIMELINE_TEXT_MAX, TimelineEntry, TimelineItem, handle_label, one_line,
    tool_summary,
};
pub use view::{
    AgentView, ERROR_CAPACITY, FEED_CAPACITY, MAX_PENDING, SpawnInfo, TIMELINE_CAPACITY,
    WorkspaceView,
};
