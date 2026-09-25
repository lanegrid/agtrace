//! Workspace-wide feed of inter-agent messages, spawns and lifecycle changes.

use agtrace_types::{
    AgentHandle, AgentId, AgentKind, AgentMessageKind, LifecycleTransition, MessageDirection,
};
use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

/// A message is usually seen twice: outgoing in the sender's log and incoming in the
/// recipient's log. The recipient records it when it reads it, which can be long
/// after the send (a busy Claude teammate reads queued messages at its next turn;
/// a Codex child records its task when it starts). The two sides are paired
/// one-to-one (oldest unpaired side first) when the incoming side is at most this
/// much later than the outgoing side ...
pub const MESSAGE_DELIVERY_WINDOW: Duration = Duration::hours(1);
/// ... or at most this much earlier (the sender may log a message only once its
/// send call returned, after the recipient already logged it).
pub const MESSAGE_CLOCK_SKEW: Duration = Duration::seconds(30);
/// Reports of the same lifecycle transition of one agent (e.g. a Claude subagent's
/// `SubagentHandback` and the later task-notification, both "completed") are merged
/// when they are this close and nothing addressed the agent in between.
pub const LIFECYCLE_DEDUPE_WINDOW: Duration = Duration::seconds(60);

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
    /// Events folded into this entry (the other side of a message, repeated
    /// lifecycle reports). Replays of these events add nothing.
    pub merged: Vec<Uuid>,
}

impl FeedEntry {
    /// True if `self` has a plaintext body and `other` does not.
    pub(crate) fn has_better_body_than(&self, other: &FeedEntry) -> bool {
        let plain = |e: &FeedEntry| e.text.is_some() && !e.encrypted;
        plain(self) && !plain(other)
    }

    /// True if `id` is this entry's event or one folded into it.
    pub(crate) fn covers(&self, id: &Uuid) -> bool {
        self.event_id == *id || self.merged.contains(id)
    }

    /// Fold `other` (a duplicate of `self`) into `self`, keeping the plaintext body.
    pub(crate) fn absorb(&mut self, other: FeedEntry) {
        if other.has_better_body_than(self) {
            self.text = other.text;
            self.encrypted = other.encrypted;
        } else if self.text.is_none() && !self.encrypted {
            self.text = other.text;
        }
        for id in std::iter::once(other.event_id).chain(other.merged) {
            if !self.covers(&id) {
                self.merged.push(id);
            }
        }
    }

    /// Timing rule for pairing the two sides of a message (see
    /// [`MESSAGE_DELIVERY_WINDOW`]); false unless one side is outgoing and the other incoming.
    pub(crate) fn delivery_plausible(&self, other: &FeedEntry) -> bool {
        let (sent, read) = match (self.direction, other.direction) {
            (Some(MessageDirection::Outgoing), Some(MessageDirection::Incoming)) => {
                (self.ts, other.ts)
            }
            (Some(MessageDirection::Incoming), Some(MessageDirection::Outgoing)) => {
                (other.ts, self.ts)
            }
            _ => return false,
        };
        let delay = read - sent;
        delay >= -MESSAGE_CLOCK_SKEW && delay <= MESSAGE_DELIVERY_WINDOW
    }

    /// Both bodies could be the same message: equal (one may be truncated or
    /// missing / encrypted).
    pub(crate) fn body_compatible(&self, other: &FeedEntry) -> bool {
        fn plain(e: &FeedEntry) -> Option<&str> {
            e.text
                .as_deref()
                .filter(|_| !e.encrypted)
                .map(|t| t.trim_end_matches('…').trim_end())
        }
        match (plain(self), plain(other)) {
            (Some(a), Some(b)) => a.starts_with(b) || b.starts_with(a),
            _ => true,
        }
    }
}
