use std::path::PathBuf;
use std::sync::Arc;

use crate::error::{Error, Result};
use crate::watch::WatchBuilder;

pub struct Client {
    inner: Arc<agtrace_runtime::AgTrace>,
}

impl Client {
    pub fn connect(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let runtime = agtrace_runtime::AgTrace::open(path).map_err(Error::Internal)?;
        Ok(Self {
            inner: Arc::new(runtime),
        })
    }

    pub fn watch(&self) -> WatchBuilder {
        WatchBuilder::new(self.inner.clone())
    }

    pub fn session(&self, id: &str) -> SessionHandle {
        SessionHandle {
            client_inner: self.inner.clone(),
            id: id.to_string(),
        }
    }
}

pub struct SessionHandle {
    client_inner: Arc<agtrace_runtime::AgTrace>,
    id: String,
}

impl SessionHandle {
    pub fn events(&self) -> Result<Vec<agtrace_types::event::AgentEvent>> {
        let session_handle = self
            .client_inner
            .sessions()
            .find(&self.id)
            .map_err(|e| Error::NotFound(format!("Session {}: {}", self.id, e)))?;

        session_handle.events().map_err(Error::Internal)
    }

    pub fn summary(&self) -> Result<agtrace_engine::SessionSummary> {
        let events = self.events()?;
        let session = agtrace_engine::assemble_session(&events)
            .ok_or_else(|| Error::Internal(anyhow::anyhow!("Failed to assemble session")))?;
        Ok(agtrace_engine::session::summarize(&session))
    }
}
