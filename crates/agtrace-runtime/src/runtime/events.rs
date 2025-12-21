use agtrace_engine::AgentSession;
use agtrace_index::SessionSummary;
use agtrace_types::AgentEvent;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum DiscoveryEvent {
    NewSession {
        summary: SessionSummary,
    },
    SessionUpdated {
        session_id: String,
        provider_name: String,
        is_new: bool,
    },
    SessionRemoved {
        session_id: String,
    },
}

#[derive(Debug, Clone)]
pub enum StreamEvent {
    Attached {
        session_id: String,
        path: PathBuf,
    },
    Events {
        events: Vec<AgentEvent>,
        session: Option<AgentSession>,
    },
    Disconnected {
        reason: String,
    },
}

#[derive(Debug, Clone)]
pub enum WorkspaceEvent {
    Discovery(DiscoveryEvent),
    Stream(StreamEvent),
    Error(String),
}
