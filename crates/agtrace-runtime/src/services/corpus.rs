use agtrace_engine::assemble_session;
use agtrace_index::Database;
use anyhow::Result;

use crate::session_repository::{LoadOptions, SessionRepository};

pub struct CorpusStats {
    pub sample_size: usize,
    pub total_tool_calls: usize,
    pub total_failures: usize,
    pub max_duration_ms: i64,
}

pub struct CorpusService<'a> {
    db: &'a Database,
}

impl<'a> CorpusService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn get_overview(&self, project_hash: Option<&str>, limit: usize) -> Result<CorpusStats> {
        let raw_sessions = self.db.list_sessions(project_hash, limit)?;

        let loader = SessionRepository::new(self.db);
        let options = LoadOptions::default();

        let mut total_tool_calls = 0;
        let mut total_failures = 0;
        let mut max_duration = 0i64;

        for session in &raw_sessions {
            if let Ok(events) = loader.load_events(&session.id, &options) {
                if let Some(agent_session) = assemble_session(&events) {
                    for turn in &agent_session.turns {
                        for step in &turn.steps {
                            total_tool_calls += step.tools.len();
                            for tool_exec in &step.tools {
                                if tool_exec.is_error {
                                    total_failures += 1;
                                }
                            }
                        }
                        if turn.stats.duration_ms > max_duration {
                            max_duration = turn.stats.duration_ms;
                        }
                    }
                }
            }
        }

        Ok(CorpusStats {
            sample_size: raw_sessions.len(),
            total_tool_calls,
            total_failures,
            max_duration_ms: max_duration,
        })
    }
}
