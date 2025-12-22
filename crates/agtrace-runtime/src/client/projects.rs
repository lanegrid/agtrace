use crate::ops::{IndexProgress, IndexService, ProjectInfo, ProjectService};
use agtrace_index::Database;
use agtrace_providers::{ProviderAdapter, ScanContext};
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
        let db = self.db.lock().unwrap();
        let service = ProjectService::new(&db);
        service.list_projects()
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
