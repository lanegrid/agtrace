use crate::ops::{
    CorpusStats, IndexProgress, IndexService, StatsResult, collect_tool_stats, get_corpus_overview,
};
use agtrace_index::Database;
use agtrace_providers::{ProviderAdapter, ScanContext};
use agtrace_types::{discover_project_root, project_hash_from_root};
use anyhow::Result;
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

    pub fn corpus_stats(&self, project_hash: Option<&str>, limit: usize) -> Result<CorpusStats> {
        self.ensure_index_is_fresh()?;

        let db = self.db.lock().unwrap();
        get_corpus_overview(&db, project_hash, limit)
    }

    pub fn tool_usage(&self, limit: Option<usize>, source: Option<String>) -> Result<StatsResult> {
        self.ensure_index_is_fresh()?;

        let db = self.db.lock().unwrap();
        collect_tool_stats(&db, limit, source)
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

        // Scan all projects: project_root=None means no filtering
        // project_hash is only used for reporting and as fallback
        let project_hash = discover_project_root(None)
            .ok()
            .map(|root| project_hash_from_root(&root.display().to_string()))
            .unwrap_or_else(|| "unknown".to_string());

        let scan_context = ScanContext {
            project_hash,
            project_root: None,
        };

        service.run(&scan_context, false, |_progress: IndexProgress| {})?;

        Ok(())
    }
}
