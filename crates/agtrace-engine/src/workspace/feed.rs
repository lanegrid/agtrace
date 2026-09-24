//! Workspace-wide feed of inter-agent messages, spawns and lifecycle changes.

use agtrace_types::{
    AgentHandle, AgentId, AgentKind, AgentMessageKind, LifecycleTransition, MessageDirection,
};
use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

/// Messages seen from both sides (outgoing in the sender's log, incoming in the
/// recipient's log) are merged when their timestamps are this close.
pub const FEED_DEDUPE_WINDOW: Duration = Duration::seconds(2);

/// One side of a feed entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeedParty {
    Agent(AgentId),
    User,
    /// Handle that does not (yet) resolve to a known agent; re-resolved when agents appear.
    Unresolved {
        handle: AgentHandle,
        label: String,
    },
}

impl FeedParty {
    pub fn agent(&self) -> Option<&AgentId> {
        match self {
            FeedParty::Agent(id) => Some(id),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeedKind {
    Message(AgentMessageKind),
    Spawn(AgentKind),
    Lifecycle(LifecycleTransition),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FeedEntry {
    /// Event the entry was built from (the first side seen, for merged messages).
    pub event_id: Uuid,
    pub ts: DateTime<Utc>,
    /// Agent whose log contained the event.
    pub source: AgentId,
    /// Sender (message), spawner (spawn) or subject (lifecycle).
    pub from: FeedParty,
    /// Recipients (message) or the spawned child; empty for lifecycle entries.
    pub to: Vec<FeedParty>,
    pub kind: FeedKind,
    /// Message body / summary, lifecycle reason, or spawn description (one line).
    pub text: Option<String>,
    pub encrypted: bool,
    pub direction: Option<MessageDirection>,
}

impl FeedEntry {
    /// True if `self` has a plaintext body and `other` does not.
    pub(crate) fn has_better_body_than(&self, other: &FeedEntry) -> bool {
        let plain = |e: &FeedEntry| e.text.is_some() && !e.encrypted;
        plain(self) && !plain(other)
    }

    pub(crate) fn within_window(&self, other: &FeedEntry) -> bool {
        (self.ts - other.ts).abs() <= FEED_DEDUPE_WINDOW
    }
}
