use agtrace_index::Database;
use agtrace_providers::{normalize_claude_file, normalize_codex_file, normalize_gemini_file};
use agtrace_types::AgentEvent;
use anyhow::Result;
use std::path::Path;

pub struct SessionLoader<'a> {
    db: &'a Database,
}

#[derive(Default)]
pub struct LoadOptions {}

impl<'a> SessionLoader<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn load_events(&self, session_id: &str, _options: &LoadOptions) -> Result<Vec<AgentEvent>> {
        let resolved_id = self.resolve_session_id(session_id)?;
        let log_files = self.db.get_session_files(&resolved_id)?;

        if log_files.is_empty() {
            anyhow::bail!("Session not found: {}", session_id);
        }

        let mut all_events = Vec::new();

        for log_file in &log_files {
            let path = Path::new(&log_file.path);

            // Call provider-specific normalization functions
            let result = if log_file.path.contains(".claude/") {
                normalize_claude_file(path)
            } else if log_file.path.contains(".codex/") {
                normalize_codex_file(path)
            } else if log_file.path.contains(".gemini/") {
                normalize_gemini_file(path)
            } else {
                anyhow::bail!("Cannot detect provider from path: {}", log_file.path)
            };

            match result {
                Ok(mut events) => {
                    all_events.append(&mut events);
                }
                Err(e) => {
                    eprintln!("Warning: Failed to normalize {}: {}", log_file.path, e);
                }
            }
        }

        all_events.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        Ok(all_events)
    }

    fn resolve_session_id(&self, session_id: &str) -> Result<String> {
        match self.db.find_session_by_prefix(session_id)? {
            Some(full_id) => Ok(full_id),
            None => {
                let files = self.db.get_session_files(session_id)?;
                if files.is_empty() {
                    anyhow::bail!("Session not found: {}", session_id);
                }
                Ok(session_id.to_string())
            }
        }
    }
}
