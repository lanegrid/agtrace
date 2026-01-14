use agtrace_types::{ProjectHash, RepositoryHash, SpawnContext};

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
    /// Provider name (claude, codex, gemini).
    pub provider: String,
    /// Session start timestamp (ISO 8601).
    pub start_ts: Option<String>,
    /// Session end timestamp (ISO 8601), if completed.
    pub end_ts: Option<String>,
    /// First user message snippet for display.
    pub snippet: Option<String>,
    /// Whether the session was successfully parsed and validated.
    pub is_valid: bool,
    /// Parent session ID for subagent sessions.
    pub parent_session_id: Option<String>,
    /// Spawn context for subagent sessions (turn/step where spawned).
    pub spawned_by: Option<SpawnContext>,
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
    /// File role (main, metadata, etc.).
    pub role: String,
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
    /// Provider name (claude, codex, gemini).
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
    /// Parent session ID for subagent sessions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_session_id: Option<String>,
    /// Spawn context for subagent sessions (turn/step where spawned).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spawned_by: Option<SpawnContext>,
}
