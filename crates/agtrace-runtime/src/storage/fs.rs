use agtrace_index::Database;
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct RawFileContent {
    pub path: String,
    pub content: String,
}

pub fn get_raw_files(db: &Database, session_id: &str) -> Result<Vec<RawFileContent>> {
    let log_files = db.get_session_files(session_id)?;

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
