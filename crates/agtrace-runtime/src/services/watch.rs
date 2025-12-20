use crate::runtime::{Runtime, RuntimeConfig};
use agtrace_providers::LogProvider;
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

pub struct WatchService;

impl WatchService {
    pub fn watch_session(
        provider: Arc<dyn LogProvider>,
        log_root: PathBuf,
        session_id: String,
        project_root: Option<PathBuf>,
    ) -> Result<Runtime> {
        Runtime::start(RuntimeConfig {
            provider,
            watch_path: log_root,
            explicit_target: Some(session_id),
            project_root,
            poll_interval: Duration::from_millis(500),
            reactors: vec![],
        })
    }
}
