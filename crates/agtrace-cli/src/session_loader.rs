use agtrace_index::Database;
use agtrace_providers::{detect_provider_from_path, ImportContext, LogProvider};
use agtrace_types::AgentEventV1;
use anyhow::Result;
use std::path::Path;

pub struct SessionLoader<'a> {
    db: &'a Database,
}

#[derive(Default)]
pub struct LoadOptions {
    pub include_sidechain: bool,
}

impl<'a> SessionLoader<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn load_events(
        &self,
        session_id: &str,
        options: &LoadOptions,
    ) -> Result<Vec<AgentEventV1>> {
        let resolved_id = self.resolve_session_id(session_id)?;
        let log_files = self.db.get_session_files(&resolved_id)?;

        if log_files.is_empty() {
            anyhow::bail!("Session not found: {}", session_id);
        }

        let files_to_process: Vec<_> = if options.include_sidechain {
            log_files
        } else {
            log_files
                .into_iter()
                .filter(|f| f.role != "sidechain")
                .collect()
        };

        if files_to_process.is_empty() {
            anyhow::bail!("No log files found for session: {}", session_id);
        }

        let mut all_events = Vec::new();

        for log_file in &files_to_process {
            let path = Path::new(&log_file.path);
            let provider = self.detect_provider(&log_file.path)?;

            let context = ImportContext {
                project_root_override: None,
                session_id_prefix: None,
                all_projects: false,
            };

            match provider.normalize_file(path, &context) {
                Ok(mut events) => {
                    all_events.append(&mut events);
                }
                Err(e) => {
                    eprintln!("Warning: Failed to normalize {}: {}", log_file.path, e);
                }
            }
        }

        all_events.sort_by(|a, b| a.ts.cmp(&b.ts));

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

    fn detect_provider(&self, path: &str) -> Result<Box<dyn LogProvider>> {
        detect_provider_from_path(path)
    }
}
