use crate::Result;
use agtrace_engine::{SessionDigest, analyze_and_select_sessions, assemble_session};
use agtrace_index::{Database, SessionSummary};
use std::collections::HashMap;

use crate::storage::{LoadOptions, SessionRepository};

pub struct PackResult {
    pub selections: Vec<SessionDigest>,
    pub balanced_count: usize,
    pub raw_count: usize,
}

pub struct PackService<'a> {
    db: &'a Database,
}

impl<'a> PackService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn select_sessions(
        &self,
        project_hash: Option<&agtrace_types::ProjectHash>,
        limit: usize,
    ) -> Result<PackResult> {
        let raw_sessions = self.db.list_sessions(project_hash, Some(1000))?;
        let balanced_sessions = balance_sessions_by_provider(&raw_sessions, 200);

        let mut digests = Vec::new();
        let loader = SessionRepository::new(self.db);
        let options = LoadOptions::default();

        for (i, session) in balanced_sessions.iter().enumerate() {
            if let Ok(events) = loader.load_events(&session.id, &options)
                && let Some(agent_session) = assemble_session(&events)
            {
                let recency_boost = (balanced_sessions.len() - i) as u32;
                let digest = SessionDigest::new(
                    &session.id,
                    &session.provider,
                    agent_session,
                    recency_boost,
                );
                digests.push(digest);
            }
        }

        let selections = analyze_and_select_sessions(digests, limit);

        Ok(PackResult {
            selections,
            balanced_count: balanced_sessions.len(),
            raw_count: raw_sessions.len(),
        })
    }
}

fn balance_sessions_by_provider(
    sessions: &[SessionSummary],
    target_per_provider: usize,
) -> Vec<SessionSummary> {
    let mut by_provider: HashMap<String, Vec<SessionSummary>> = HashMap::new();
    for session in sessions {
        by_provider
            .entry(session.provider.clone())
            .or_default()
            .push(session.clone());
    }

    let mut balanced = Vec::new();
    for (_, mut list) in by_provider {
        if list.len() > target_per_provider {
            list.truncate(target_per_provider);
        }
        balanced.extend(list);
    }

    balanced.sort_by(|a, b| b.start_ts.cmp(&a.start_ts));
    balanced
}
