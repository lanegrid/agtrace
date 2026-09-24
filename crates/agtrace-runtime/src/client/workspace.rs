use crate::client::{InsightOps, MonitorBuilder, ProjectOps, SessionOps, WatchService};
use crate::config::Config;
use crate::init::{InitConfig, InitProgress, InitResult, InitService};
use crate::model_catalog::ConfiguredModelCatalog;
use crate::ops::{CheckResult, DoctorService, InspectResult};
use crate::workspace::{WatchRoots, WatchScope, WatcherOptions, WorkspaceWatcher};
use crate::{Error, Result};
use agtrace_engine::DiagnoseResult;
use agtrace_index::Database;
use agtrace_providers::ProviderAdapter;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::task;

pub struct AgTrace {
    db: Arc<Mutex<Database>>,
    config: Arc<Config>,
    provider_configs: Arc<Vec<(String, PathBuf)>>,
    model_catalog: Arc<ConfiguredModelCatalog>,
}

impl AgTrace {
    pub fn setup<F>(config: InitConfig, progress_fn: Option<F>) -> Result<InitResult>
    where
        F: FnMut(InitProgress),
    {
        InitService::run(config, progress_fn)
    }

    pub async fn connect_or_create(data_dir: PathBuf) -> Result<Self> {
        let db_path = data_dir.join("agtrace.db");
        let config_path = data_dir.join("config.toml");

        if !data_dir.exists() {
            std::fs::create_dir_all(&data_dir)?;
        }

        let config = if tokio::fs::try_exists(&config_path).await? {
            Config::load_from(&config_path)?
        } else {
            let detected = Config::detect_providers()?;
            detected.save_to(&config_path)?;
            detected
        };

        let db = task::spawn_blocking(move || Database::open(&db_path).map_err(Error::Index))
            .await
            .map_err(|e| Error::InvalidOperation(format!("Task join error: {}", e)))??;

        let provider_configs: Vec<(String, PathBuf)> = config
            .enabled_providers()
            .into_iter()
            .map(|(name, cfg)| (name.clone(), cfg.log_root.clone()))
            .collect();

        let model_catalog = ConfiguredModelCatalog::load(config.context_window.clone());

        Ok(Self {
            db: Arc::new(Mutex::new(db)),
            config: Arc::new(config),
            provider_configs: Arc::new(provider_configs),
            model_catalog: Arc::new(model_catalog),
        })
    }

    pub async fn open(data_dir: PathBuf) -> Result<Self> {
        let db_path = data_dir.join("agtrace.db");
        let config_path = data_dir.join("config.toml");

        // Database operations wrapped in spawn_blocking
        let db = task::spawn_blocking(move || {
            Database::open(&db_path).map_err(|e| {
                if !db_path.exists() {
                    Error::NotInitialized(format!(
                        "Database not found. Please run 'agtrace init' to initialize the workspace.\n\
                         Database path: {}",
                        db_path.display()
                    ))
                } else {
                    Error::Index(e)
                }
            })
        })
        .await
        .map_err(|e| Error::InvalidOperation(format!("Task join error: {}", e)))??;

        // File I/O can use tokio::fs
        let config = if tokio::fs::try_exists(&config_path).await? {
            Config::load_from(&config_path)?
        } else {
            let detected = Config::detect_providers()?;
            detected.save_to(&config_path)?;
            detected
        };

        let provider_configs: Vec<(String, PathBuf)> = config
            .enabled_providers()
            .into_iter()
            .map(|(name, cfg)| (name.clone(), cfg.log_root.clone()))
            .collect();

        let model_catalog = ConfiguredModelCatalog::load(config.context_window.clone());

        Ok(Self {
            db: Arc::new(Mutex::new(db)),
            config: Arc::new(config),
            provider_configs: Arc::new(provider_configs),
            model_catalog: Arc::new(model_catalog),
        })
    }

    pub fn diagnose(&self) -> Result<Vec<DiagnoseResult>> {
        let providers: Vec<(ProviderAdapter, PathBuf)> = self
            .provider_configs
            .iter()
            .filter_map(|(name, path)| {
                agtrace_providers::create_adapter(name)
                    .ok()
                    .map(|p| (p, path.clone()))
            })
            .collect();
        DoctorService::diagnose_all(&providers)
    }

    pub fn projects(&self) -> ProjectOps {
        ProjectOps::new(self.db.clone(), self.provider_configs.clone())
    }

    pub fn sessions(&self) -> SessionOps {
        SessionOps::new(self.db.clone(), self.provider_configs.clone())
    }

    pub fn insights(&self) -> InsightOps {
        InsightOps::new(self.db.clone(), self.provider_configs.clone())
    }

    pub fn watch_service(&self) -> WatchService {
        WatchService::new(
            self.db.clone(),
            self.config.clone(),
            self.provider_configs.clone(),
            self.watch_roots(),
        )
    }

    /// Legacy session-update feed (old `watch` UI); see [`WatchService`].
    pub fn workspace_monitor(&self) -> Result<MonitorBuilder> {
        Ok(MonitorBuilder::new(self.watch_roots()))
    }

    /// Provider directories for the workspace watcher, for the enabled providers.
    ///
    /// `AGTRACE_CLAUDE_HOME` / `AGTRACE_CODEX_HOME` win (tests, demo); otherwise the
    /// configured log roots are used (`<claude home>/projects`, `<codex home>/sessions`),
    /// and the Claude home (registry, teams) is the parent of the projects root.
    pub fn watch_roots(&self) -> WatchRoots {
        let log_root = |name: &str| {
            self.provider_configs
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, p)| p.clone())
        };
        let env_set = |var: &str| std::env::var_os(var).is_some_and(|v| !v.is_empty());
        let mut roots = WatchRoots::default();
        if let Some(projects) = log_root("claude_code") {
            if env_set(agtrace_core::CLAUDE_HOME_ENV) {
                roots.claude_home = agtrace_core::claude_home();
                roots.claude_projects = agtrace_core::claude_projects_root();
            } else {
                roots.claude_home = projects.parent().map(PathBuf::from);
                roots.claude_projects = Some(projects);
            }
        }
        if let Some(sessions) = log_root("codex") {
            roots.codex_sessions = if env_set(agtrace_core::CODEX_HOME_ENV) {
                agtrace_core::codex_sessions_root()
            } else {
                Some(sessions)
            };
        }
        roots
    }

    /// Start the workspace watcher (design §4.2) for `scope`.
    pub fn watch_workspace(&self, scope: WatchScope) -> Result<WorkspaceWatcher> {
        WorkspaceWatcher::start(scope, self.watch_roots(), WatcherOptions::default())
    }

    pub fn database(&self) -> Arc<Mutex<Database>> {
        self.db.clone()
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Model catalog for the context window resolver (built-in tables, provider caches,
    /// `[context_window]` user overrides).
    pub fn model_catalog(&self) -> Arc<ConfiguredModelCatalog> {
        self.model_catalog.clone()
    }

    pub fn check_file(
        file_path: &str,
        provider: &ProviderAdapter,
        provider_name: &str,
    ) -> Result<CheckResult> {
        DoctorService::check_file(file_path, provider, provider_name)
    }

    pub fn inspect_file(file_path: &str, lines: usize, json_format: bool) -> Result<InspectResult> {
        DoctorService::inspect_file(file_path, lines, json_format)
    }
}
