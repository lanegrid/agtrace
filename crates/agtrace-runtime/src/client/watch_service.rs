use crate::client::{MonitorBuilder, StreamHandle};
use crate::config::Config;
use crate::runtime::SessionStreamer;
use agtrace_index::Database;
use agtrace_types::project_hash_from_root;
use anyhow::Result;
use std::path::{Path, PathBuf};
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
                    Err(e) => last_error = Some(e),
                }
            }

            return Err(last_error
                .unwrap_or_else(|| anyhow::anyhow!("No providers configured"))
                .context(format!("Session not found: {}", resolved_id)));
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

    pub fn watch_all_providers(&self) -> Result<MonitorBuilder> {
        Ok(MonitorBuilder::new(
            self.db.clone(),
            self.provider_configs.clone(),
        ))
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn database(&self) -> Arc<Mutex<Database>> {
        self.db.clone()
    }

    /// Find the provider with the most recently updated session.
    ///
    /// If `project_root` is specified, only sessions from that project are considered.
    /// Otherwise, searches across all projects.
    ///
    /// Returns the provider name with the most recent session, or None if no sessions found.
    ///
    /// # Design Rationale
    /// - Watch mode needs real-time detection of "most recently updated" sessions
    /// - Cannot rely on DB indexing since watch bypasses DB for real-time monitoring
    /// - Directly scans filesystem using LogDiscovery::scan_sessions()
    /// - Uses SessionIndex.latest_mod_time (file modification time) instead of timestamp (creation time)
    /// - This enables switching to sessions that are actively being updated, even if created earlier
    /// - Filters by project_root to ensure watch attaches to sessions in the current project only
    pub fn find_most_recent_provider(&self, project_root: Option<&Path>) -> Option<String> {
        // Calculate target project hash if project_root is specified
        let target_project_hash =
            project_root.map(|root| project_hash_from_root(&root.display().to_string()));

        // Track the most recently updated session across all providers
        let mut most_recent: Option<(String, String)> = None; // (provider_name, latest_mod_time)

        for (provider_name, log_root) in self.provider_configs.iter() {
            // Create adapter for this provider
            let adapter = match agtrace_providers::create_adapter(provider_name) {
                Ok(a) => a,
                Err(_) => continue,
            };

            // Scan filesystem directly (bypassing DB for real-time detection)
            let sessions = match adapter.discovery.scan_sessions(log_root) {
                Ok(s) => s,
                Err(_) => continue,
            };

            // Find the session with the latest modification time in this provider
            for session in sessions {
                // Filter by project if project_root is specified
                if let Some(ref target_hash) = target_project_hash {
                    let session_hash = session
                        .project_root
                        .as_ref()
                        .map(|root| project_hash_from_root(root));
                    if session_hash.as_ref() != Some(target_hash) {
                        continue;
                    }
                }

                if let Some(ref mod_time) = session.latest_mod_time
                    && (most_recent.is_none()
                        || Some(mod_time) > most_recent.as_ref().map(|(_, t)| t))
                {
                    most_recent = Some((provider_name.clone(), mod_time.clone()));
                }
            }
        }

        most_recent.map(|(provider, _)| provider)
    }
}
