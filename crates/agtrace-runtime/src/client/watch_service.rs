use crate::client::{MonitorBuilder, StreamHandle};
use crate::config::Config;
use crate::runtime::SessionStreamer;
use agtrace_index::Database;
use anyhow::Result;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct WatchService {
    db: Arc<Mutex<Database>>,
    config: Arc<Config>,
    provider_configs: Arc<Vec<(String, PathBuf)>>,
}

impl WatchService {
    pub fn new(
        db: Arc<Mutex<Database>>,
        config: Arc<Config>,
        provider_configs: Arc<Vec<(String, PathBuf)>>,
    ) -> Self {
        Self {
            db,
            config,
            provider_configs,
        }
    }

    pub fn watch_session(&self, session_id: &str) -> Result<StreamHandle> {
        let session_opt = {
            let db = self.db.lock().unwrap();
            db.get_session_by_id(session_id)?
        };

        let streamer = if let Some(session) = session_opt {
            // Session exists in database, use normal attach
            let adapter = agtrace_providers::create_adapter(&session.provider)?;
            SessionStreamer::attach(session_id.to_string(), self.db.clone(), Arc::new(adapter))?
        } else {
            // Session not in database yet, scan filesystem directly
            // Try each configured provider until we find the session
            let mut last_error = None;

            for (provider_name, log_root) in self.provider_configs.iter() {
                match agtrace_providers::create_adapter(provider_name) {
                    Ok(adapter) => {
                        match SessionStreamer::attach_from_filesystem(
                            session_id.to_string(),
                            log_root.clone(),
                            Arc::new(adapter),
                        ) {
                            Ok(streamer) => return Ok(StreamHandle::new(streamer)),
                            Err(e) => last_error = Some(e),
                        }
                    }
                    Err(e) => last_error = Some(e),
                }
            }

            return Err(last_error
                .unwrap_or_else(|| anyhow::anyhow!("No providers configured"))
                .context(format!("Session not found: {}", session_id)));
        };

        Ok(StreamHandle::new(streamer))
    }

    pub fn watch_provider(&self, provider_name: &str) -> Result<MonitorBuilder> {
        let log_root = self
            .provider_configs
            .iter()
            .find(|(name, _)| name == provider_name)
            .map(|(_, path)| path.clone())
            .ok_or_else(|| anyhow::anyhow!("Provider '{}' not configured", provider_name))?;

        Ok(MonitorBuilder::new(
            self.db.clone(),
            Arc::new(vec![(provider_name.to_string(), log_root)]),
        ))
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn database(&self) -> Arc<Mutex<Database>> {
        self.db.clone()
    }
}
