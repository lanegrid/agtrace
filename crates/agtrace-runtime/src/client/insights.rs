use crate::ops::{collect_tool_stats, get_corpus_overview, CorpusStats, StatsResult};
use agtrace_index::Database;
use anyhow::Result;
use std::sync::Arc;

pub struct InsightOps {
    db: Arc<Database>,
}

impl InsightOps {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub fn corpus_stats(&self, project_hash: Option<&str>, limit: usize) -> Result<CorpusStats> {
        get_corpus_overview(&self.db, project_hash, limit)
    }

    pub fn tool_usage(&self, limit: Option<usize>, source: Option<String>) -> Result<StatsResult> {
        collect_tool_stats(&self.db, limit, source)
    }
}
