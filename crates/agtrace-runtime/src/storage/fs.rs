use agtrace_index::Database;
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct RawFileContent {
    pub path: String,
    pub content: String,
}

pub fn get_raw_files(db: &Database, session_id: &str) -> Result<Vec<RawFileContent>> {
    // Resolve short session ID to full ID if needed
    let resolved_id = if session_id.len() < 36 {
        db.find_session_by_prefix(session_id)?
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?
    } else {
        session_id.to_string()
    };

    let log_files = db.get_session_files(&resolved_id)?;

    let mut contents = Vec::new();
    for log_file in &log_files {
        let content = std::fs::read_to_string(&log_file.path)?;
        contents.push(RawFileContent {
            path: log_file.path.clone(),
            content,
        });
    }

    Ok(contents)
}
