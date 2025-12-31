use crate::Result;
use crate::ops::{
    CorpusStats, IndexProgress, IndexService, StatsResult, collect_tool_stats, get_corpus_overview,
};
use agtrace_index::Database;
use agtrace_providers::ProviderAdapter;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct InsightOps {
    db: Arc<Mutex<Database>>,
    provider_configs: Arc<Vec<(String, PathBuf)>>,
}

impl InsightOps {
    pub fn new(db: Arc<Mutex<Database>>, provider_configs: Arc<Vec<(String, PathBuf)>>) -> Self {
        Self {
            db,
            provider_configs,
        }
    }

    pub fn corpus_stats(
        &self,
        project_hash: Option<&agtrace_types::ProjectHash>,
        limit: usize,
    ) -> Result<CorpusStats> {
        self.ensure_index_is_fresh()?;

        let db = self.db.lock().unwrap();
        get_corpus_overview(&db, project_hash, limit)
    }

    pub fn tool_usage(
        &self,
        limit: Option<usize>,
        provider: Option<String>,
    ) -> Result<StatsResult> {
        self.ensure_index_is_fresh()?;

        let db = self.db.lock().unwrap();
        collect_tool_stats(&db, limit, provider)
    }

    fn ensure_index_is_fresh(&self) -> Result<()> {
        let db = self.db.lock().unwrap();

        let providers: Vec<(ProviderAdapter, PathBuf)> = self
            .provider_configs
            .iter()
            .filter_map(|(name, path)| {
                agtrace_providers::create_adapter(name)
                    .ok()
                    .map(|p| (p, path.clone()))
            })
            .collect();

        let service = IndexService::new(&db, providers);

        // Scan all projects without filtering
        let scope = agtrace_types::ProjectScope::All;

        service.run(scope, false, |_progress: IndexProgress| {})?;

        Ok(())
    }
}
