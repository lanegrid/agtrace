use crate::config::Config;
use crate::runtime::{Runtime, RuntimeConfig, RuntimeEvent, TokenUsageMonitor};
use anyhow::Result;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::sync::Arc;

pub struct RuntimeBuilder {
    #[allow(dead_code)]
    config: Arc<Config>,
    provider_configs: Arc<Vec<(String, PathBuf)>>,
    explicit_target: Option<String>,
    project_root: Option<PathBuf>,
    enable_token_monitor: bool,
    provider_name: Option<String>,
}

impl RuntimeBuilder {
    pub fn new(config: Arc<Config>, provider_configs: Arc<Vec<(String, PathBuf)>>) -> Self {
        Self {
            config,
            provider_configs,
            explicit_target: None,
            project_root: None,
            enable_token_monitor: false,
            provider_name: None,
        }
    }

    pub fn watch_session(mut self, session_id: &str) -> Self {
        self.explicit_target = Some(session_id.to_string());
        self.enable_token_monitor = false;
        self
    }

    pub fn watch_latest(mut self) -> Self {
        self.explicit_target = None;
        self.enable_token_monitor = true;
        self
    }

    pub fn with_provider(mut self, provider_name: &str) -> Self {
        self.provider_name = Some(provider_name.to_string());
        self
    }

    pub fn with_project_root(mut self, project_root: PathBuf) -> Self {
        self.project_root = Some(project_root);
        self
    }

    pub fn with_token_monitor(mut self) -> Self {
        self.enable_token_monitor = true;
        self
    }

    pub fn start(self) -> Result<ActiveRuntime> {
        let (provider_name, log_root) = if let Some(name) = &self.provider_name {
            self.provider_configs
                .iter()
                .find(|(n, _)| n == name)
                .map(|(n, path)| (n.clone(), path.clone()))
                .ok_or_else(|| anyhow::anyhow!("Provider '{}' not found", name))?
        } else {
            self.provider_configs
                .first()
                .map(|(n, path)| (n.clone(), path.clone()))
                .ok_or_else(|| anyhow::anyhow!("No providers available"))?
        };

        let provider = agtrace_providers::create_provider(&provider_name)?;

        let mut reactors: Vec<Box<dyn crate::runtime::reactor::Reactor>> = vec![];

        if self.enable_token_monitor {
            reactors.push(Box::new(TokenUsageMonitor::default_thresholds()));
        }

        let runtime = Runtime::start(RuntimeConfig {
            provider: Arc::from(provider),
            reactors,
            watch_path: log_root,
            explicit_target: self.explicit_target,
            project_root: self.project_root,
        })?;

        Ok(ActiveRuntime { runtime })
    }
}

pub struct ActiveRuntime {
    runtime: Runtime,
}

impl ActiveRuntime {
    pub fn receiver(&self) -> &Receiver<RuntimeEvent> {
        self.runtime.receiver()
    }

    pub fn next_event(&self) -> Option<RuntimeEvent> {
        self.runtime.receiver().recv().ok()
    }
}
