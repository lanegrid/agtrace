use crate::config_service::Config;
use agtrace_index::{Database, SessionSummary};
use agtrace_providers::get_all_providers;
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub enum InitEvent {
    Header,
    Step1Detecting,
    Step1Loading,
    Step1DetectedProviders {
        providers: HashMap<String, PathBuf>,
        config_saved: bool,
    },
    Step1LoadedConfig {
        config_path: PathBuf,
    },
    Step1NoProvidersDetected {
        available_providers: Vec<ProviderInfo>,
    },
    Step2Header,
    Step2DbReady {
        db_path: PathBuf,
    },
    Step3Header,
    Step3ScanCompleted {
        success: bool,
        error: Option<String>,
    },
    Step3Skipped {
        reason: SkipReason,
    },
    Step4Header,
    Step4NoSessions {
        all_projects: bool,
    },
    Step4SessionsFound {
        sessions: Vec<SessionSummary>,
        all_projects: bool,
    },
    NextSteps {
        session_id: String,
    },
}

#[derive(Debug, Clone)]
pub struct ProviderInfo {
    pub name: String,
    pub default_log_path: String,
}

#[derive(Debug, Clone)]
pub enum SkipReason {
    RecentlyScanned { elapsed: Duration },
}

pub struct InitConfig {
    pub data_dir: PathBuf,
    pub project_root: Option<PathBuf>,
    pub all_projects: bool,
    pub refresh: bool,
}

pub struct InitService;

impl InitService {
    pub fn run<F>(config: InitConfig, mut event_fn: F) -> Result<()>
    where
        F: FnMut(InitEvent) -> Result<()>,
    {
        event_fn(InitEvent::Header)?;

        let config_path = config.data_dir.join("config.toml");
        let db_path = config.data_dir.join("agtrace.db");

        let cfg = Self::step1_config(&config_path, &mut event_fn)?;

        if cfg.is_none() {
            return Ok(());
        }

        let db = Self::step2_database(&db_path, &mut event_fn)?;

        let current_project_root = config
            .project_root
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| ".".to_string());
        let current_project_hash = agtrace_types::project_hash_from_root(&current_project_root);

        Self::step3_scan(&db, &current_project_hash, config.refresh, &mut event_fn)?;

        Self::step4_sessions(
            &db,
            &current_project_hash,
            config.all_projects,
            &mut event_fn,
        )?;

        Ok(())
    }

    fn step1_config<F>(config_path: &Path, event_fn: &mut F) -> Result<Option<Config>>
    where
        F: FnMut(InitEvent) -> Result<()>,
    {
        if !config_path.exists() {
            event_fn(InitEvent::Step1Detecting)?;
            let detected = Config::detect_providers()?;

            if detected.providers.is_empty() {
                let available_providers = get_all_providers()
                    .iter()
                    .map(|p| ProviderInfo {
                        name: p.name.to_string(),
                        default_log_path: p.default_log_path.to_string(),
                    })
                    .collect();
                event_fn(InitEvent::Step1NoProvidersDetected {
                    available_providers,
                })?;
                return Ok(None);
            }

            let providers: HashMap<String, PathBuf> = detected
                .providers
                .iter()
                .map(|(name, cfg)| (name.clone(), cfg.log_root.clone()))
                .collect();

            detected.save_to(&config_path.to_path_buf())?;
            event_fn(InitEvent::Step1DetectedProviders {
                providers,
                config_saved: true,
            })?;

            Ok(Some(detected))
        } else {
            event_fn(InitEvent::Step1Loading)?;
            let cfg = Config::load_from(&config_path.to_path_buf())?;
            event_fn(InitEvent::Step1LoadedConfig {
                config_path: config_path.to_path_buf(),
            })?;
            Ok(Some(cfg))
        }
    }

    fn step2_database<F>(db_path: &Path, event_fn: &mut F) -> Result<Database>
    where
        F: FnMut(InitEvent) -> Result<()>,
    {
        event_fn(InitEvent::Step2Header)?;
        let db = Database::open(db_path)?;
        event_fn(InitEvent::Step2DbReady {
            db_path: db_path.to_path_buf(),
        })?;
        Ok(db)
    }

    fn step3_scan<F>(
        db: &Database,
        project_hash: &str,
        force_refresh: bool,
        event_fn: &mut F,
    ) -> Result<()>
    where
        F: FnMut(InitEvent) -> Result<()>,
    {
        event_fn(InitEvent::Step3Header)?;

        let should_scan = if force_refresh {
            true
        } else if let Ok(Some(project)) = db.get_project(project_hash) {
            if let Some(last_scanned) = &project.last_scanned_at {
                if let Ok(last_time) = DateTime::parse_from_rfc3339(last_scanned) {
                    let elapsed = Utc::now().signed_duration_since(last_time.with_timezone(&Utc));
                    if elapsed < Duration::minutes(5) {
                        event_fn(InitEvent::Step3Skipped {
                            reason: SkipReason::RecentlyScanned { elapsed },
                        })?;
                        false
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
            event_fn(InitEvent::Step3ScanCompleted {
                success: true,
                error: None,
            })?;
        }

        Ok(())
    }

    fn step4_sessions<F>(
        db: &Database,
        project_hash: &str,
        all_projects: bool,
        event_fn: &mut F,
    ) -> Result<()>
    where
        F: FnMut(InitEvent) -> Result<()>,
    {
        event_fn(InitEvent::Step4Header)?;

        let effective_hash = if all_projects {
            None
        } else {
            Some(project_hash)
        };

        let sessions = db.list_sessions(effective_hash, 10)?;

        if sessions.is_empty() {
            event_fn(InitEvent::Step4NoSessions { all_projects })?;
            return Ok(());
        }

        event_fn(InitEvent::Step4SessionsFound {
            sessions: sessions.clone(),
            all_projects,
        })?;

        if let Some(first_session) = sessions.first() {
            event_fn(InitEvent::NextSteps {
                session_id: first_session.id.clone(),
            })?;
        }

        Ok(())
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

        let mut events = Vec::new();
        InitService::run(config, |event| {
            events.push(format!("{:?}", event));
            Ok(())
        })?;

        assert!(!events.is_empty());
        assert!(events[0].contains("Header"));

        Ok(())
    }
}
