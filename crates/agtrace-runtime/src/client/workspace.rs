use crate::client::{InsightOps, ProjectOps, RuntimeBuilder, SessionOps};
use crate::config::Config;
use crate::init::{InitConfig, InitProgress, InitResult, InitService};
use crate::ops::{CheckResult, DoctorService, InspectResult};
use agtrace_engine::DiagnoseResult;
use agtrace_index::Database;
use agtrace_providers::LogProvider;
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;

pub struct AgTrace {
    db: Arc<Database>,
    config: Arc<Config>,
    provider_configs: Arc<Vec<(String, PathBuf)>>,
}

impl AgTrace {
    pub fn setup<F>(config: InitConfig, progress_fn: Option<F>) -> Result<InitResult>
    where
        F: FnMut(InitProgress),
    {
        InitService::run(config, progress_fn)
    }

    pub fn open(data_dir: PathBuf) -> Result<Self> {
        let db_path = data_dir.join("agtrace.db");
        let config_path = data_dir.join("config.toml");

        let db = Database::open(&db_path)?;
        let config = if config_path.exists() {
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

        #[allow(clippy::arc_with_non_send_sync)]
        Ok(Self {
            db: Arc::new(db),
            config: Arc::new(config),
            provider_configs: Arc::new(provider_configs),
        })
    }

    pub fn diagnose(&self) -> Result<Vec<DiagnoseResult>> {
        let providers: Vec<(Box<dyn LogProvider>, PathBuf)> = self
            .provider_configs
            .iter()
            .filter_map(|(name, path)| {
                agtrace_providers::create_provider(name)
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
        SessionOps::new(self.db.clone())
    }

    pub fn insights(&self) -> InsightOps {
        InsightOps::new(self.db.clone())
    }

    pub fn monitor(&self) -> RuntimeBuilder {
        RuntimeBuilder::new(self.config.clone(), self.provider_configs.clone())
    }

    pub fn database(&self) -> &Database {
        &self.db
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn check_file(
        file_path: &str,
        provider: &dyn LogProvider,
        provider_name: &str,
    ) -> Result<CheckResult> {
        DoctorService::check_file(file_path, provider, provider_name)
    }

    pub fn inspect_file(file_path: &str, lines: usize, json_format: bool) -> Result<InspectResult> {
        DoctorService::inspect_file(file_path, lines, json_format)
    }
}
