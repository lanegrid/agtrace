use agtrace_types::{ProjectHash, RepositoryHash};

/// Project metadata record from the index database.
///
/// Stores project-level information discovered during scanning.
#[derive(Debug, Clone)]
pub struct ProjectRecord {
    /// Project identifier (hash of root path).
    pub hash: ProjectHash,
    /// Absolute path to project root directory, if known.
    pub root_path: Option<String>,
    /// Last time this project was scanned (ISO 8601 timestamp).
    pub last_scanned_at: Option<String>,
}

/// Complete session record from the index database.
///
/// Contains all indexed metadata for a session, including validity status.
/// Used internally by the index layer.
#[derive(Debug, Clone)]
pub struct SessionRecord {
    /// Session UUID.
    pub id: String,
    /// Project this session belongs to.
    pub project_hash: ProjectHash,
    /// Git repository hash for worktree support (None for non-git directories).
    pub repository_hash: Option<RepositoryHash>,
    /// Provider name (claude, codex).
    pub provider: String,
    /// Session start timestamp (ISO 8601).
    pub start_ts: Option<String>,
    /// Session end timestamp (ISO 8601), if completed.
    pub end_ts: Option<String>,
    /// First user message snippet for display.
    pub snippet: Option<String>,
    /// Whether the session was successfully parsed and validated.
    pub is_valid: bool,
    /// Agent kind of the file owner: `main`, `teammate`, `codex_thread`, `fork`.
    pub agent_kind: String,
    /// Display name (teammate name, Codex agent_path leaf / nickname).
    pub agent_name: Option<String>,
    /// Hierarchical path (Codex `agent_path`, e.g. `/root/judge`).
    pub agent_path: Option<String>,
    /// Claude Agent Teams team name.
    pub team_name: Option<String>,
    /// Root session of the tree (Codex `session_id`; Claude lead for teammates when known).
    pub root_session_id: Option<String>,
    /// Parent session ID (Codex parent thread; Claude team lead). No FK: children may be
    /// indexed before their parent.
    pub parent_session_id: Option<String>,
    /// Provider call id of the spawning tool call in the parent.
    pub spawn_call_id: Option<String>,
}

/// Log file metadata record from the index database.
///
/// Tracks individual log files that contribute to sessions.
#[derive(Debug, Clone)]
pub struct LogFileRecord {
    /// Absolute path to the log file.
    pub path: String,
    /// Session UUID this file belongs to.
    pub session_id: String,
    /// File role: `main` (session owner), `subagent` or `fork` (Claude `subagents/`).
    pub role: String,
    /// Agent id (`claude:<sid>`, `claude:<sid>/<aid>`, `codex:<thread>`).
    pub agent_id: String,
    /// Agent display name (subagents: meta.json name / description).
    pub agent_name: Option<String>,
    /// Provider call id of the spawning tool call (subagents: meta.json `toolUseId`).
    pub spawn_call_id: Option<String>,
    /// File size in bytes.
    pub file_size: Option<i64>,
    /// File modification time (ISO 8601 timestamp).
    pub mod_time: Option<String>,
}

/// Lightweight session summary for list operations.
///
/// Returned by session listing APIs. Contains only the essential
/// information needed for session selection and preview.
/// This is the primary type SDK users interact with when browsing sessions.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionSummary {
    /// Session UUID.
    pub id: String,
    /// Provider name (claude, codex).
    pub provider: String,
    /// Project this session belongs to.
    pub project_hash: ProjectHash,
    /// Git repository hash for worktree support (None for non-git directories).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository_hash: Option<RepositoryHash>,
    /// Absolute path to project root directory, if known.
    pub project_root: Option<String>,
    /// Session start timestamp (ISO 8601).
    pub start_ts: Option<String>,
    /// First user message snippet for display.
    pub snippet: Option<String>,
    /// Agent kind of the file owner: `main`, `teammate`, `codex_thread`, `fork`.
    #[serde(default)]
    pub agent_kind: String,
    /// Display name (teammate name, Codex agent_path leaf / nickname).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_name: Option<String>,
    /// Hierarchical path (Codex `agent_path`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_path: Option<String>,
    /// Claude Agent Teams team name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub team_name: Option<String>,
    /// Root session of the tree this session belongs to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_session_id: Option<String>,
    /// Parent session ID (Codex parent thread; Claude team lead).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_session_id: Option<String>,
    /// Provider call id of the spawning tool call in the parent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spawn_call_id: Option<String>,
}
