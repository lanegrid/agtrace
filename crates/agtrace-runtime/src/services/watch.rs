use crate::runtime::{Runtime, RuntimeConfig};
use crate::token_usage_monitor::TokenUsageMonitor;
use agtrace_providers::LogProvider;
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

pub struct WatchConfig {
    pub provider: Arc<dyn LogProvider>,
    pub log_root: PathBuf,
    pub explicit_target: Option<String>,
    pub project_root: Option<PathBuf>,
    pub enable_token_monitor: bool,
}

pub struct WatchService;

impl WatchService {
    pub fn start(config: WatchConfig) -> Result<Runtime> {
        let mut reactors: Vec<Box<dyn crate::Reactor>> = vec![];

        if config.enable_token_monitor {
            reactors.push(Box::new(TokenUsageMonitor::default_thresholds()));
        }

        Runtime::start(RuntimeConfig {
            provider: config.provider,
            watch_path: config.log_root,
            explicit_target: config.explicit_target,
            project_root: config.project_root,
            poll_interval: Duration::from_millis(500),
            reactors,
        })
    }

    pub fn watch_session(
        provider: Arc<dyn LogProvider>,
        log_root: PathBuf,
        session_id: String,
        project_root: Option<PathBuf>,
    ) -> Result<Runtime> {
        Self::start(WatchConfig {
            provider,
            log_root,
            explicit_target: Some(session_id),
            project_root,
            enable_token_monitor: false,
        })
    }

    pub fn watch_provider(
        provider: Arc<dyn LogProvider>,
        log_root: PathBuf,
        project_root: Option<PathBuf>,
    ) -> Result<Runtime> {
        Self::start(WatchConfig {
            provider,
            log_root,
            explicit_target: None,
            project_root,
            enable_token_monitor: true,
        })
    }
}
