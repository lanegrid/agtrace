use crate::config::Config;
use crate::ops::IndexService;
use agtrace_index::Database;
use agtrace_providers::get_all_providers;
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub enum InitProgress {
    ConfigPhase,
    DatabasePhase,
    ScanPhase,
    SessionPhase,
}

#[derive(Debug, Clone)]
pub struct ProviderInfo {
    pub name: String,
    pub default_log_path: String,
}

#[derive(Debug, Clone)]
pub enum ConfigStatus {
    DetectedAndSaved {
        providers: HashMap<String, PathBuf>,
    },
    LoadedExisting {
        config_path: PathBuf,
    },
    NoProvidersDetected {
        available_providers: Vec<ProviderInfo>,
    },
}

#[derive(Debug, Clone)]
pub enum ScanOutcome {
    Scanned,
    Skipped { elapsed: Duration },
}

#[derive(Debug, Clone)]
pub struct InitResult {
    pub config_status: ConfigStatus,
    pub db_path: PathBuf,
    pub scan_outcome: ScanOutcome,
    pub session_count: usize,
    pub all_projects: bool,
    pub scan_needed: bool,
}

pub struct InitConfig {
    pub data_dir: PathBuf,
    pub project_root: Option<PathBuf>,
    pub all_projects: bool,
    pub refresh: bool,
}

pub struct InitService;

impl InitService {
    pub fn run<F>(config: InitConfig, mut progress_fn: Option<F>) -> Result<InitResult>
    where
        F: FnMut(InitProgress),
    {
        let config_path = config.data_dir.join("config.toml");
        let db_path = config.data_dir.join("agtrace.db");

        if let Some(ref mut f) = progress_fn {
            f(InitProgress::ConfigPhase);
        }
        let config_status = Self::step1_config(&config_path)?;

        if let ConfigStatus::NoProvidersDetected { .. } = config_status {
            return Ok(InitResult {
                config_status,
                db_path: db_path.clone(),
                scan_outcome: ScanOutcome::Skipped {
                    elapsed: Duration::zero(),
                },
                session_count: 0,
                all_projects: config.all_projects,
                scan_needed: false,
            });
        }

        if let Some(ref mut f) = progress_fn {
            f(InitProgress::DatabasePhase);
        }
        let db = Self::step2_database(&db_path)?;

        let current_project_root = config
            .project_root
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| ".".to_string());
        let current_project_hash = agtrace_types::project_hash_from_root(&current_project_root);

        if let Some(ref mut f) = progress_fn {
            f(InitProgress::ScanPhase);
        }
        let (scan_outcome, scan_needed) =
            Self::step3_scan(&db, &current_project_hash, config.refresh)?;

        // Perform actual scan if needed
        if scan_needed {
            let loaded_config = Config::load_from(&config_path)?;
            let providers: Vec<(agtrace_providers::ProviderAdapter, PathBuf)> = loaded_config
                .providers
                .iter()
                .filter_map(|(name, cfg)| {
                    agtrace_providers::create_adapter(name)
                        .ok()
                        .map(|p| (p, cfg.log_root.clone()))
                })
                .collect();

            let service = IndexService::new(&db, providers);
            let scope = if config.all_projects {
                agtrace_types::ProjectScope::All
            } else {
                agtrace_types::ProjectScope::Specific {
                    root: current_project_root.clone(),
                }
            };

            service.run(scope, config.refresh, |_progress| {
                // Silently index during init - progress is shown by the handler
            })?;
        }

        if let Some(ref mut f) = progress_fn {
            f(InitProgress::SessionPhase);
        }
        let session_count = Self::step4_sessions(&db, &current_project_hash, config.all_projects)?;

        Ok(InitResult {
            config_status,
            db_path: db_path.clone(),
            scan_outcome,
            session_count,
            all_projects: config.all_projects,
            scan_needed,
        })
    }

    fn step1_config(config_path: &Path) -> Result<ConfigStatus> {
        if !config_path.exists() {
            let detected = Config::detect_providers()?;

            if detected.providers.is_empty() {
                let available_providers = get_all_providers()
                    .iter()
                    .map(|p| ProviderInfo {
                        name: p.name.to_string(),
                        default_log_path: p.default_log_path.to_string(),
                    })
                    .collect();
                return Ok(ConfigStatus::NoProvidersDetected {
                    available_providers,
                });
            }

            let providers: HashMap<String, PathBuf> = detected
                .providers
                .iter()
                .map(|(name, cfg)| (name.clone(), cfg.log_root.clone()))
                .collect();

            detected.save_to(&config_path.to_path_buf())?;

            Ok(ConfigStatus::DetectedAndSaved { providers })
        } else {
            Config::load_from(&config_path.to_path_buf())?;
            Ok(ConfigStatus::LoadedExisting {
                config_path: config_path.to_path_buf(),
            })
        }
    }

    fn step2_database(db_path: &Path) -> Result<Database> {
        Database::open(db_path)
    }

    fn step3_scan(
        db: &Database,
        project_hash: &str,
        force_refresh: bool,
    ) -> Result<(ScanOutcome, bool)> {
        let should_scan = if force_refresh {
            true
        } else if let Ok(Some(project)) = db.get_project(project_hash) {
            if let Some(last_scanned) = &project.last_scanned_at {
                if let Ok(last_time) = DateTime::parse_from_rfc3339(last_scanned) {
                    let elapsed = Utc::now().signed_duration_since(last_time.with_timezone(&Utc));
                    if elapsed < Duration::minutes(5) {
                        return Ok((ScanOutcome::Skipped { elapsed }, false));
                    } else {
                        true
                    }
                } else {
                    true
                }
            } else {
                true
            }
        } else {
            true
        };

        if should_scan {
            Ok((ScanOutcome::Scanned, true))
        } else {
            Ok((
                ScanOutcome::Skipped {
                    elapsed: Duration::zero(),
                },
                false,
            ))
        }
    }

    fn step4_sessions(db: &Database, project_hash: &str, all_projects: bool) -> Result<usize> {
        let effective_hash = if all_projects {
            None
        } else {
            Some(project_hash)
        };

        let sessions = db.list_sessions(effective_hash, 10)?;
        Ok(sessions.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_init_service_basic_flow() -> Result<()> {
        let temp_dir = TempDir::new()?;

        let config = InitConfig {
            data_dir: temp_dir.path().to_path_buf(),
            project_root: None,
            all_projects: false,
            refresh: false,
        };

        let result = InitService::run(config, None::<fn(InitProgress)>)?;

        matches!(
            result.config_status,
            ConfigStatus::NoProvidersDetected { .. }
        );

        Ok(())
    }
}
