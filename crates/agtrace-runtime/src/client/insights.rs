use crate::ops::{collect_tool_stats, get_corpus_overview, CorpusStats, StatsResult};
use agtrace_index::Database;
use anyhow::Result;
use std::sync::{Arc, Mutex};

pub struct InsightOps {
    db: Arc<Mutex<Database>>,
}

impl InsightOps {
    pub fn new(db: Arc<Mutex<Database>>) -> Self {
        Self { db }
    }

    pub fn corpus_stats(&self, project_hash: Option<&str>, limit: usize) -> Result<CorpusStats> {
        let db = self.db.lock().unwrap();
        get_corpus_overview(&db, project_hash, limit)
    }

    pub fn tool_usage(&self, limit: Option<usize>, source: Option<String>) -> Result<StatsResult> {
        let db = self.db.lock().unwrap();
        collect_tool_stats(&db, limit, source)
    }
}
