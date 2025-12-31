use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures::stream::Stream;

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
            .map_err(crate::error::Error::Runtime)?
            .start_background_scan()
            .map_err(crate::error::Error::Runtime)?;

        // Create async channel for stream implementation
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        // Spawn background task to bridge blocking receiver to async sender
        // The monitor is moved into the task
        tokio::task::spawn_blocking(move || {
            while let Ok(event) = monitor.receiver().recv() {
                if tx.send(event).is_err() {
                    break; // Receiver dropped
                }
            }
        });

        Ok(LiveStream { receiver: rx })
    }
}

pub struct LiveStream {
    receiver: tokio::sync::mpsc::UnboundedReceiver<WorkspaceEvent>,
}

impl LiveStream {
    /// Poll for the next event (non-blocking).
    ///
    /// Returns `None` if no event is available immediately.
    pub fn try_next(&mut self) -> Option<WorkspaceEvent> {
        self.receiver.try_recv().ok()
    }
}

impl Stream for LiveStream {
    type Item = WorkspaceEvent;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.receiver.poll_recv(cx)
    }
}
