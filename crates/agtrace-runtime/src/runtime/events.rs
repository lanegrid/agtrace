use agtrace_engine::AgentSession;
use agtrace_index::SessionSummary;
use agtrace_types::AgentEvent;
use std::path::PathBuf;

/// NOTE: DiscoveryEvent design for real-time session tracking
/// - SessionUpdated.is_new: True if first time seeing this session_id in this process
/// - SessionUpdated.mod_time: File modification timestamp for "most recently updated" detection
/// - Watch handlers use mod_time to switch to actively updated sessions, not just is_new
#[derive(Debug, Clone)]
pub enum DiscoveryEvent {
    NewSession {
        summary: SessionSummary,
    },
    SessionUpdated {
        session_id: String,
        provider_name: String,
        is_new: bool,
        mod_time: Option<String>,
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
        /// Assembled sessions (main + child streams).
        /// Runtime performs assembly; consumers should not call assemble_sessions directly.
        sessions: Vec<AgentSession>,
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
