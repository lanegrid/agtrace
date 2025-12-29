use crate::runtime::{SessionStreamer, WatchContext, WorkspaceEvent, WorkspaceSupervisor};
use agtrace_index::Database;
use anyhow::Result;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};

pub struct MonitorBuilder {
    db: Arc<Mutex<Database>>,
    provider_configs: Arc<Vec<(String, PathBuf)>>,
    project_root: Option<PathBuf>,
}

impl MonitorBuilder {
    pub fn new(db: Arc<Mutex<Database>>, provider_configs: Arc<Vec<(String, PathBuf)>>) -> Self {
        Self {
            db,
            provider_configs,
            project_root: None,
        }
    }

    pub fn with_project_root(mut self, project_root: PathBuf) -> Self {
        self.project_root = Some(project_root);
        self
    }

    pub fn start_background_scan(self) -> Result<WorkspaceMonitor> {
        let mut contexts = Vec::new();

        for (provider_name, root) in self.provider_configs.iter() {
            if let Ok(adapter) = agtrace_providers::create_adapter(provider_name) {
                contexts.push(WatchContext {
                    provider_name: provider_name.clone(),
                    provider: Arc::new(adapter),
                    root: root.clone(),
                });
            }
        }

        let supervisor = WorkspaceSupervisor::start(contexts, self.db.clone(), self.project_root)?;

        Ok(WorkspaceMonitor {
            db: self.db,
            supervisor,
            provider_configs: self.provider_configs,
        })
    }
}

pub struct WorkspaceMonitor {
    db: Arc<Mutex<Database>>,
    supervisor: WorkspaceSupervisor,
    provider_configs: Arc<Vec<(String, PathBuf)>>,
}

impl WorkspaceMonitor {
    pub fn receiver(&self) -> &Receiver<WorkspaceEvent> {
        self.supervisor.receiver()
    }

    pub fn next_event(&self) -> Option<WorkspaceEvent> {
        self.supervisor.receiver().recv().ok()
    }

    pub fn attach(&self, session_id: &str, provider_name: Option<&str>) -> Result<StreamHandle> {
        let provider_name = if let Some(name) = provider_name {
            name.to_string()
        } else {
            self.provider_configs
                .first()
                .map(|(n, _)| n.clone())
                .ok_or_else(|| anyhow::anyhow!("No providers available"))?
        };

        let adapter = agtrace_providers::create_adapter(&provider_name)?;

        let streamer =
            SessionStreamer::attach(session_id.to_string(), self.db.clone(), Arc::new(adapter))?;

        Ok(StreamHandle { streamer })
    }
}

pub struct StreamHandle {
    streamer: SessionStreamer,
}

impl StreamHandle {
    pub(crate) fn new(streamer: SessionStreamer) -> Self {
        Self { streamer }
    }

    pub fn receiver(&self) -> &Receiver<WorkspaceEvent> {
        self.streamer.receiver()
    }
}
