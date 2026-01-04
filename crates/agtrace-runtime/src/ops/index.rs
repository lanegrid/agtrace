use crate::{Error, Result};
use agtrace_index::{Database, LogFileRecord, ProjectRecord, SessionRecord};
use agtrace_providers::ProviderAdapter;
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

    pub fn run<F>(
        &self,
        scope: agtrace_types::ProjectScope,
        force: bool,
        mut on_progress: F,
    ) -> Result<()>
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
                .discovery
                .scan_sessions(log_root)
                .map_err(Error::Provider)?;

            // Filter sessions by project_hash if specified
            let filtered_sessions: Vec<_> = sessions
                .into_iter()
                .filter(|session| {
                    if let Some(expected_hash) = scope.hash() {
                        if let Some(session_root) = &session.project_root {
                            let session_hash = agtrace_core::project_hash_from_root(&session_root.to_string_lossy());
                            &session_hash == expected_hash
                        } else {
                            // Gemini sessions might not have project_root, compute hash from file
                            if provider_name == "gemini" {
                                use agtrace_providers::gemini::io::extract_project_hash_from_gemini_file;
                                if let Some(session_hash) = extract_project_hash_from_gemini_file(&session.main_file) {
                                    &session_hash == expected_hash
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        }
                    } else {
                        true
                    }
                })
                .collect();

            on_progress(IndexProgress::ProviderSessionCount {
                provider_name: provider_name.to_string(),
                count: filtered_sessions.len(),
                project_hash: match &scope {
                    agtrace_types::ProjectScope::All => "<all>".to_string(),
                    agtrace_types::ProjectScope::Specific(hash) => hash.to_string(),
                },
                all_projects: matches!(scope, agtrace_types::ProjectScope::All),
            });

            for session in filtered_sessions {
                // Collect all file paths for this session
                let mut all_files = vec![session.main_file.display().to_string()];
                for side_file in &session.sidechain_files {
                    all_files.push(side_file.display().to_string());
                }

                let all_files_unchanged =
                    !force && all_files.iter().all(|f| indexed_files.contains(f));

                if all_files_unchanged {
                    skipped_files += all_files.len();
                    continue;
                }

                on_progress(IndexProgress::SessionRegistered {
                    session_id: session.session_id.clone(),
                });

                // Calculate project_hash from session data
                let session_project_hash = if let Some(ref root) = session.project_root {
                    agtrace_core::project_hash_from_root(&root.to_string_lossy())
                } else if provider_name == "gemini" {
                    // For Gemini, extract project_hash directly from the file
                    use agtrace_providers::gemini::io::extract_project_hash_from_gemini_file;
                    extract_project_hash_from_gemini_file(&session.main_file).unwrap_or_else(|| {
                        agtrace_core::project_hash_from_log_path(&session.main_file)
                    })
                } else {
                    // Generate unique hash from log path for orphaned sessions
                    agtrace_core::project_hash_from_log_path(&session.main_file)
                };

                let project_record = ProjectRecord {
                    hash: session_project_hash.clone(),
                    root_path: session
                        .project_root
                        .as_ref()
                        .map(|p| p.to_string_lossy().to_string()),
                    last_scanned_at: Some(chrono::Utc::now().to_rfc3339()),
                };
                self.db.insert_or_update_project(&project_record)?;

                let session_record = SessionRecord {
                    id: session.session_id.clone(),
                    project_hash: session_project_hash,
                    provider: provider_name.to_string(),
                    start_ts: session.timestamp.clone(),
                    end_ts: None,
                    snippet: session.snippet.clone(),
                    is_valid: true,
                    parent_session_id: session.parent_session_id.clone(),
                    spawned_by: session.spawned_by.clone(),
                };
                self.db.insert_or_update_session(&session_record)?;

                // Register main file
                let to_log_file_record = |path: &PathBuf, role: &str| -> Result<LogFileRecord> {
                    let meta = std::fs::metadata(path).ok();
                    Ok(LogFileRecord {
                        path: path.display().to_string(),
                        session_id: session.session_id.clone(),
                        role: role.to_string(),
                        file_size: meta.as_ref().map(|m| m.len() as i64),
                        mod_time: meta
                            .and_then(|m| m.modified().ok())
                            .map(|t| format!("{:?}", t)),
                    })
                };

                scanned_files += 1;
                let main_log_file = to_log_file_record(&session.main_file, "main")?;
                self.db.insert_or_update_log_file(&main_log_file)?;

                // Register sidechain files
                for side_file in &session.sidechain_files {
                    scanned_files += 1;
                    let side_log_file = to_log_file_record(side_file, "sidechain")?;
                    self.db.insert_or_update_log_file(&side_log_file)?;
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
