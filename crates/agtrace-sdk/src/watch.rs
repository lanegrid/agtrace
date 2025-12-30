use std::sync::Arc;

use crate::error::Result;

// Re-export event types for convenient use in examples/client code
pub use agtrace_runtime::{DiscoveryEvent, StreamEvent, WorkspaceEvent};

pub struct WatchBuilder {
    inner: Arc<agtrace_runtime::AgTrace>,
    providers: Vec<String>,
}

impl WatchBuilder {
    pub(crate) fn new(inner: Arc<agtrace_runtime::AgTrace>) -> Self {
        Self {
            inner,
            providers: vec![],
        }
    }

    pub fn provider(mut self, name: &str) -> Self {
        self.providers.push(name.to_string());
        self
    }

    pub fn all_providers(mut self) -> Self {
        self.providers.clear();
        self
    }

    pub fn start(self) -> Result<LiveStream> {
        let monitor = self
            .inner
            .workspace_monitor()
            .map_err(crate::error::Error::Internal)?
            .start_background_scan()
            .map_err(crate::error::Error::Internal)?;
        Ok(LiveStream { monitor })
    }
}

pub struct LiveStream {
    monitor: agtrace_runtime::WorkspaceMonitor,
}

impl LiveStream {
    pub fn next_blocking(&self) -> Option<WorkspaceEvent> {
        self.monitor.receiver().recv().ok()
    }

    pub fn try_next(&self) -> Option<WorkspaceEvent> {
        self.monitor.receiver().try_recv().ok()
    }
}

impl Iterator for LiveStream {
    type Item = WorkspaceEvent;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_blocking()
    }
}
