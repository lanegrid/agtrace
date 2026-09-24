//! Legacy watch service for the old `watch` TUI / console: follow one session
//! ([`WatchService::watch_session`]) and get notified about updated sessions
//! ([`WatchService::watch_all_providers`], backed by the workspace watcher).

use crate::client::{MonitorBuilder, StreamHandle};
use crate::config::Config;
use crate::runtime::SessionStreamer;
use crate::workspace::WatchRoots;
use crate::{Error, Result};
use agtrace_index::Database;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct WatchService {
    db: Arc<Mutex<Database>>,
    config: Arc<Config>,
    provider_configs: Arc<Vec<(String, PathBuf)>>,
    roots: WatchRoots,
}

impl WatchService {
    pub fn new(
        db: Arc<Mutex<Database>>,
        config: Arc<Config>,
        provider_configs: Arc<Vec<(String, PathBuf)>>,
        roots: WatchRoots,
    ) -> Self {
        Self {
            db,
            config,
            provider_configs,
            roots,
        }
    }

    pub fn watch_session(&self, session_id: &str) -> Result<StreamHandle> {
        // Try to resolve short ID to full ID from database
        // If not found, use the provided session_id as-is (might be a full ID for a new session)
        let resolved_id = {
            let db = self.db.lock().unwrap();
            if let Some(session) = db.get_session_by_id(session_id)? {
                session.id
            } else if let Some(full_id) = db.find_session_by_prefix(session_id)? {
                full_id
            } else {
                // Not in database - use as-is and let filesystem scan handle it
                session_id.to_string()
            }
        };

        let session_opt = {
            let db = self.db.lock().unwrap();
            db.get_session_by_id(&resolved_id)?
        };

        let streamer = if let Some(session) = session_opt {
            // Session exists in database, use normal attach
            let adapter = agtrace_providers::create_adapter(&session.provider)?;
            SessionStreamer::attach(resolved_id.clone(), self.db.clone(), Arc::new(adapter))?
        } else {
            // Session not in database yet, scan filesystem directly
            // Try each configured provider until we find the session
            let mut last_error = None;

            for (provider_name, log_root) in self.provider_configs.iter() {
                match agtrace_providers::create_adapter(provider_name) {
                    Ok(adapter) => {
                        match SessionStreamer::attach_from_filesystem(
                            resolved_id.clone(),
                            log_root.clone(),
                            Arc::new(adapter),
                        ) {
                            Ok(streamer) => return Ok(StreamHandle::new(streamer)),
                            Err(e) => last_error = Some(e),
                        }
                    }
                    Err(e) => last_error = Some(Error::Provider(e)),
                }
            }

            return Err(last_error.unwrap_or_else(|| {
                Error::InvalidOperation(format!("Session not found: {}", resolved_id))
            }));
        };

        Ok(StreamHandle::new(streamer))
    }

    /// Session-update feed for one provider.
    pub fn watch_provider(&self, provider_name: &str) -> Result<MonitorBuilder> {
        let mut roots = self.roots.clone();
        match agtrace_types::Provider::from_name(provider_name) {
            Some(agtrace_types::Provider::ClaudeCode) => roots.codex_sessions = None,
            Some(agtrace_types::Provider::Codex) => {
                roots.claude_projects = None;
                roots.claude_home = None;
            }
            None => {
                return Err(Error::InvalidOperation(format!(
                    "Provider '{}' not configured",
                    provider_name
                )));
            }
        }
        Ok(MonitorBuilder::new(roots))
    }

    /// Session-update feed for every enabled provider.
    pub fn watch_all_providers(&self) -> Result<MonitorBuilder> {
        Ok(MonitorBuilder::new(self.roots.clone()))
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn database(&self) -> Arc<Mutex<Database>> {
        self.db.clone()
    }
}
