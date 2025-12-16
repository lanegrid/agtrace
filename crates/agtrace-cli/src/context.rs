use crate::config::Config;
use agtrace_index::Database;
use agtrace_providers::{create_provider, LogProvider};
use anyhow::{anyhow, Result};
use once_cell::sync::OnceCell;
use std::path::{Path, PathBuf};

pub struct ExecutionContext {
    data_dir: PathBuf,
    db: OnceCell<Database>,
    config: OnceCell<Config>,
    pub project_root: Option<PathBuf>,
    pub all_projects: bool,
}

impl ExecutionContext {
    pub fn new(
        data_dir: PathBuf,
        project_root: Option<String>,
        all_projects: bool,
    ) -> Result<Self> {
        let project_root = project_root
            .map(PathBuf::from)
            .or_else(|| std::env::current_dir().ok());

        Ok(Self {
            data_dir,
            project_root,
            all_projects,
            db: OnceCell::new(),
            config: OnceCell::new(),
        })
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn db(&self) -> Result<&Database> {
        self.db.get_or_try_init(|| {
            let db_path = self.data_dir.join("db.sqlite");
            Database::open(&db_path)
        })
    }

    pub fn config(&self) -> Result<&Config> {
        self.config.get_or_try_init(|| {
            let config_path = self.data_dir.join("config.toml");
            Config::load_from(&config_path)
        })
    }

    pub fn resolve_provider(&self, provider_name: &str) -> Result<(Box<dyn LogProvider>, PathBuf)> {
        let config = self.config()?;
        let provider_config = config
            .providers
            .get(provider_name)
            .ok_or_else(|| anyhow!("Provider '{}' not found in config", provider_name))?;

        if !provider_config.enabled {
            anyhow::bail!("Provider '{}' is not enabled", provider_name);
        }

        let provider = create_provider(provider_name)?;
        Ok((provider, provider_config.log_root.clone()))
    }

    pub fn default_provider(&self) -> Result<String> {
        let config = self.config()?;
        config
            .providers
            .iter()
            .find(|(_, p)| p.enabled)
            .map(|(name, _)| name.clone())
            .ok_or_else(|| {
                anyhow!("No enabled provider found. Run 'agtrace init' to configure providers.")
            })
    }
}
