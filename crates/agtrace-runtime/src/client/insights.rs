use crate::ops::{collect_tool_stats, get_corpus_overview, CorpusStats, StatsResult};
use agtrace_index::Database;
use anyhow::Result;
use std::path::PathBuf;

pub struct InsightOps {
    db_path: PathBuf,
}

impl InsightOps {
    pub fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }

    fn open_db(&self) -> Result<Database> {
        Database::open(&self.db_path)
    }

    pub fn corpus_stats(&self, project_hash: Option<&str>, limit: usize) -> Result<CorpusStats> {
        let db = self.open_db()?;
        get_corpus_overview(&db, project_hash, limit)
    }

    pub fn tool_usage(&self, limit: Option<usize>, source: Option<String>) -> Result<StatsResult> {
        let db = self.open_db()?;
        collect_tool_stats(&db, limit, source)
    }
}
