use agtrace_index::{Database, LogFileRecord, ProjectRecord, SessionRecord};
use agtrace_providers::{ProviderAdapter, ScanContext};
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum IndexProgress {
    IncrementalHint {
        indexed_files: usize,
    },
    LogRootMissing {
        provider_name: String,
        log_root: PathBuf,
    },
    ProviderScanning {
        provider_name: String,
    },
    ProviderSessionCount {
        provider_name: String,
        count: usize,
        project_hash: String,
        all_projects: bool,
    },
    SessionRegistered {
        session_id: String,
    },
    Completed {
        total_sessions: usize,
        scanned_files: usize,
        skipped_files: usize,
    },
}

pub struct IndexService<'a> {
    db: &'a Database,
    providers: Vec<(ProviderAdapter, PathBuf)>,
}

impl<'a> IndexService<'a> {
    pub fn new(db: &'a Database, providers: Vec<(ProviderAdapter, PathBuf)>) -> Self {
        Self { db, providers }
    }

    pub fn run<F>(&self, scan_context: &ScanContext, force: bool, mut on_progress: F) -> Result<()>
    where
        F: FnMut(IndexProgress),
    {
        let indexed_files = if force {
            HashSet::new()
        } else {
            self.db
                .get_all_log_files()?
                .into_iter()
                .filter_map(|f| {
                    if should_skip_indexed_file(&f) {
                        Some(f.path)
                    } else {
                        None
                    }
                })
                .collect::<HashSet<_>>()
        };

        if !force {
            on_progress(IndexProgress::IncrementalHint {
                indexed_files: indexed_files.len(),
            });
        }

        let mut total_sessions = 0;
        let mut scanned_files = 0;
        let mut skipped_files = 0;

        for (provider, log_root) in &self.providers {
            let provider_name = provider.id();

            if !log_root.exists() {
                on_progress(IndexProgress::LogRootMissing {
                    provider_name: provider_name.to_string(),
                    log_root: log_root.clone(),
                });
                continue;
            }

            on_progress(IndexProgress::ProviderScanning {
                provider_name: provider_name.to_string(),
            });

            let sessions = provider
                .scan_legacy(log_root, scan_context)
                .with_context(|| format!("Failed to scan {}", provider_name))?;

            on_progress(IndexProgress::ProviderSessionCount {
                provider_name: provider_name.to_string(),
                count: sessions.len(),
                project_hash: scan_context.project_hash.clone(),
                all_projects: scan_context.project_root.is_none(),
            });

            for session in sessions {
                let all_files_unchanged = !force
                    && session
                        .log_files
                        .iter()
                        .all(|f| indexed_files.contains(&f.path));

                if all_files_unchanged {
                    skipped_files += session.log_files.len();
                    continue;
                }

                on_progress(IndexProgress::SessionRegistered {
                    session_id: session.session_id.clone(),
                });

                let project_record = ProjectRecord {
                    hash: session.project_hash.clone(),
                    root_path: session.project_root.clone(),
                    last_scanned_at: Some(chrono::Utc::now().to_rfc3339()),
                };
                self.db.insert_or_update_project(&project_record)?;

                let session_record = SessionRecord {
                    id: session.session_id.clone(),
                    project_hash: session.project_hash.clone(),
                    provider: session.provider.clone(),
                    start_ts: session.start_ts.clone(),
                    end_ts: session.end_ts.clone(),
                    snippet: session.snippet.clone(),
                    is_valid: true,
                };
                self.db.insert_or_update_session(&session_record)?;

                for log_file in session.log_files {
                    scanned_files += 1;
                    let log_file_record = LogFileRecord {
                        path: log_file.path,
                        session_id: session.session_id.clone(),
                        role: log_file.role,
                        file_size: log_file.file_size,
                        mod_time: log_file.mod_time,
                    };
                    self.db.insert_or_update_log_file(&log_file_record)?;
                }

                total_sessions += 1;
            }
        }

        on_progress(IndexProgress::Completed {
            total_sessions,
            scanned_files,
            skipped_files,
        });

        Ok(())
    }
}

fn should_skip_indexed_file(indexed: &LogFileRecord) -> bool {
    use std::path::Path;

    let path = Path::new(&indexed.path);

    if !path.exists() {
        return false;
    }

    let metadata = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(_) => return false,
    };

    if let Some(db_size) = indexed.file_size {
        if db_size != metadata.len() as i64 {
            return false;
        }
    } else {
        return false;
    }

    if let Some(db_mod_time) = &indexed.mod_time {
        if let Ok(fs_mod_time) = metadata.modified() {
            let fs_mod_time_str = format!("{:?}", fs_mod_time);
            if db_mod_time != &fs_mod_time_str {
                return false;
            }
        } else {
            return false;
        }
    } else {
        return false;
    }

    true
}
