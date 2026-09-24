//! Small builders for synthetic test data.
//!
//! Everything here is invented (ids, names, text); nothing is derived from real logs.
//! The fixture tree under `fixtures/` is the integration corpus; these builders are
//! for unit tests that need precise variants.
//!
//! - [`agents`]: `AgentRef` builders (Claude main / teammate / subagent / fork, Codex threads)
//! - [`events`]: an [`events::EventLog`] that emits `AgentEvent`s of one agent in file order

pub mod agents;
pub mod events;

pub use agents::AgentBuilder;
pub use events::{EventLog, handle, ts};
