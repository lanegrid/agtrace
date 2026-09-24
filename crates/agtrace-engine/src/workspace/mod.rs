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
//!   whose team config names a lead is linked to it. Unlinked children are shown
//!   under their root.
//! - **Status**: see [`status`] for the priority table.
//! - **Feed**: inter-agent messages (deduplicated across sender / recipient logs),
//!   spawns and lifecycle changes.

mod context_seam;
mod feed;
mod input;
mod ring;
pub mod status;
mod timeline;
mod view;

pub use context_seam::{CatalogResolver, ContextEvidence, ContextWindow, NoWindow, WindowResolver};
pub use feed::{FEED_DEDUPE_WINDOW, FeedEntry, FeedKind, FeedParty};
pub use input::{ProcessStatus, SideStateUpdate, TeamMember, WorkspaceEvent};
pub use ring::RingBuffer;
pub use status::{AgentStatus, StatusSource};
pub use timeline::{
    RunningTool, TIMELINE_TEXT_MAX, TimelineEntry, TimelineItem, handle_label, one_line,
    tool_summary,
};
pub use view::{
    AgentView, ERROR_CAPACITY, FEED_CAPACITY, MAX_PENDING, SpawnInfo, TIMELINE_CAPACITY,
    WorkspaceView,
};
