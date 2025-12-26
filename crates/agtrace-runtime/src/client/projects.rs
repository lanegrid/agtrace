use crate::ops::{IndexProgress, IndexService, ProjectInfo, ProjectService};
use agtrace_index::Database;
use agtrace_providers::{ProviderAdapter, ScanContext};
use agtrace_types::{discover_project_root, project_hash_from_root};
use anyhow::Result;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct ScanSummary {
    pub total_sessions: usize,
    pub scanned_files: usize,
    pub skipped_files: usize,
}

pub struct ProjectOps {
    db: Arc<Mutex<Database>>,
    provider_configs: Arc<Vec<(String, PathBuf)>>,
}

impl ProjectOps {
    pub fn new(db: Arc<Mutex<Database>>, provider_configs: Arc<Vec<(String, PathBuf)>>) -> Self {
        Self {
            db,
            provider_configs,
        }
    }

    pub fn list(&self) -> Result<Vec<ProjectInfo>> {
        self.ensure_index_is_fresh()?;

        let db = self.db.lock().unwrap();
        let service = ProjectService::new(&db);
        service.list_projects()
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
            provider_filter: None,
        };

        service.run(&scan_context, false, |_progress: IndexProgress| {})?;

        Ok(())
    }

    pub fn scan<F>(
        &self,
        scan_context: &ScanContext,
        force: bool,
        mut on_progress: F,
    ) -> Result<ScanSummary>
    where
        F: FnMut(IndexProgress),
    {
        let db = self.db.lock().unwrap();
        let providers: Vec<(ProviderAdapter, PathBuf)> = self
            .provider_configs
            .iter()
            .filter_map(|(name, path)| {
                // Apply provider filter if specified
                if let Some(ref filter) = scan_context.provider_filter
                    && filter != "all" && name != filter
                {
                    return None;
                }
                agtrace_providers::create_adapter(name)
                    .ok()
                    .map(|p| (p, path.clone()))
            })
            .collect();
        let service = IndexService::new(&db, providers);

        let mut total_sessions = 0;
        let mut scanned_files = 0;
        let mut skipped_files = 0;

        service.run(scan_context, force, |progress| {
            if let IndexProgress::Completed {
                total_sessions: ts,
                scanned_files: sf,
                skipped_files: skf,
            } = progress
            {
                total_sessions = ts;
                scanned_files = sf;
                skipped_files = skf;
            }
            on_progress(progress);
        })?;

        Ok(ScanSummary {
            total_sessions,
            scanned_files,
            skipped_files,
        })
    }
}
