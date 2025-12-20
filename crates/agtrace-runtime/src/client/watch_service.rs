use crate::client::{MonitorBuilder, StreamHandle};
use crate::config::Config;
use crate::runtime::SessionStreamer;
use agtrace_index::Database;
use anyhow::Result;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct WatchService {
    db_path: PathBuf,
    config: Arc<Config>,
    provider_configs: Arc<Vec<(String, PathBuf)>>,
}

impl WatchService {
    pub fn new(
        db_path: PathBuf,
        config: Arc<Config>,
        provider_configs: Arc<Vec<(String, PathBuf)>>,
    ) -> Self {
        Self {
            db_path,
            config,
            provider_configs,
        }
    }

    pub fn watch_session(&self, session_id: &str) -> Result<StreamHandle> {
        let db = Database::open(&self.db_path)?;
        let db_mutex = Arc::new(Mutex::new(db));

        // TODO: Support provider selection instead of hardcoding "claude"
        let provider = agtrace_providers::create_provider("claude")?;

        let streamer =
            SessionStreamer::attach(session_id.to_string(), db_mutex, Arc::from(provider))?;

        Ok(StreamHandle::new(streamer))
    }

    pub fn watch_provider(&self, provider_name: &str) -> Result<MonitorBuilder> {
        let log_root = self
            .provider_configs
            .iter()
            .find(|(name, _)| name == provider_name)
            .map(|(_, path)| path.clone())
            .ok_or_else(|| anyhow::anyhow!("Provider '{}' not configured", provider_name))?;

        let db = Database::open(&self.db_path)?;
        let db_mutex = Arc::new(Mutex::new(db));

        Ok(MonitorBuilder::new(
            db_mutex,
            Arc::new(vec![(provider_name.to_string(), log_root)]),
        ))
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn db_path(&self) -> &PathBuf {
        &self.db_path
    }
}
