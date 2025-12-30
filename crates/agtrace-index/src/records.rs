use agtrace_types::ProjectHash;

#[derive(Debug, Clone)]
pub struct ProjectRecord {
    pub hash: ProjectHash,
    pub root_path: Option<String>,
    pub last_scanned_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SessionRecord {
    pub id: String,
    pub project_hash: ProjectHash,
    pub provider: String,
    pub start_ts: Option<String>,
    pub end_ts: Option<String>,
    pub snippet: Option<String>,
    pub is_valid: bool,
}

#[derive(Debug, Clone)]
pub struct LogFileRecord {
    pub path: String,
    pub session_id: String,
    pub role: String,
    pub file_size: Option<i64>,
    pub mod_time: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub provider: String,
    pub project_hash: ProjectHash,
    pub start_ts: Option<String>,
    pub snippet: Option<String>,
}
