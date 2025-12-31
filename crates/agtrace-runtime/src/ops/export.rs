use agtrace_engine::export::{self, ExportStrategy};
use agtrace_index::Database;
use agtrace_types::AgentEvent;
use crate::Result;

use crate::storage::{LoadOptions, SessionRepository};

pub struct ExportService<'a> {
    db: &'a Database,
}

impl<'a> ExportService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn export_session(
        &self,
        session_id: &str,
        strategy: ExportStrategy,
    ) -> Result<Vec<AgentEvent>> {
        let loader = SessionRepository::new(self.db);
        let options = LoadOptions::default();
        let events = loader.load_events(session_id, &options)?;
        let processed_events = export::transform(&events, strategy);
        Ok(processed_events)
    }
}
