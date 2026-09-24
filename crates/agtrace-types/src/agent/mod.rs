//! Agent identity: one agent = one log file = one timeline.

mod agent_ref;
mod id;

pub use agent_ref::{AgentKind, AgentRef};
pub use id::{AgentId, Provider};
