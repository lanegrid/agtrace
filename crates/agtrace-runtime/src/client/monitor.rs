//! Legacy session-switching feed for the old `watch` TUI / console.
//!
//! The old UI follows one session at a time and switches to the most recently
//! updated one. This adapter runs the [`WorkspaceWatcher`] (bounded discovery,
//! project scope, 2 h window) and reports every root agent (`Main` kind) that
//! produced events as a legacy [`DiscoveryEvent::SessionUpdated`] — except for the
//! initial tails of the startup burst, like the old supervisor that only reported
//! file changes. It replaces the old recursive `notify` supervisor and goes away
//! with the old UI.

use crate::Result;
use crate::runtime::{DiscoveryEvent, SessionStreamer, WatchEvent};
use crate::workspace::{WatchRoots, WatchScope, WatcherOptions, WorkspaceEvent, WorkspaceWatcher};
use agtrace_types::{AgentId, AgentKind, AgentRef};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::time::Duration;

/// A pause this long in the watcher's output ends the startup burst.
const STARTUP_QUIET: Duration = Duration::from_millis(150);

pub struct MonitorBuilder {
    roots: WatchRoots,
    project_root: Option<PathBuf>,
}

impl MonitorBuilder {
    pub fn new(roots: WatchRoots) -> Self {
        Self {
            roots,
            project_root: None,
        }
    }

    pub fn with_project_root(mut self, project_root: PathBuf) -> Self {
        self.project_root = Some(project_root);
        self
    }

    /// Start the workspace watcher and translate its events. Without a project root
    /// every project is in scope (`/`).
    pub fn start_background_scan(self) -> Result<WorkspaceMonitor> {
        let root = self.project_root.unwrap_or_else(|| PathBuf::from("/"));
        let watcher = WorkspaceWatcher::start(
            WatchScope::project(root),
            self.roots,
            WatcherOptions::default(),
        )?;
        let (tx, rx) = channel();
        std::thread::Builder::new()
            .name("legacy-watch-adapter".to_string())
            .spawn(move || {
                let mut roots: HashMap<AgentId, AgentRef> = HashMap::new();
                let mut seen: HashSet<String> = HashSet::new();
                let mut startup = true;
                loop {
                    let event = if startup {
                        match watcher.receiver().recv_timeout(STARTUP_QUIET) {
                            Ok(event) => event,
                            Err(RecvTimeoutError::Timeout) => {
                                startup = false;
                                continue;
                            }
                            Err(RecvTimeoutError::Disconnected) => break,
                        }
                    } else {
                        match watcher.receiver().recv() {
                            Ok(event) => event,
                            Err(_) => break,
                        }
                    };
                    let out = match event {
                        WorkspaceEvent::AgentDiscovered(agent)
                        | WorkspaceEvent::AgentUpdated(agent) => {
                            if agent.kind == AgentKind::Main && agent.parent.is_none() {
                                roots.insert(agent.id.clone(), agent);
                            }
                            None
                        }
                        WorkspaceEvent::Events { agent, events, .. }
                            if !events.is_empty() && !startup =>
                        {
                            roots.get(&agent).map(|r| {
                                let session_id = r.native_session_id.clone();
                                let is_new = seen.insert(session_id.clone());
                                let mod_time = std::fs::metadata(&r.file)
                                    .ok()
                                    .and_then(|m| m.modified().ok())
                                    .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339());
                                WatchEvent::Discovery(DiscoveryEvent::SessionUpdated {
                                    session_id,
                                    provider_name: r.provider.as_str().to_string(),
                                    is_new,
                                    mod_time,
                                })
                            })
                        }
                        WorkspaceEvent::Error(msg) => Some(WatchEvent::Error(msg)),
                        _ => None,
                    };
                    if let Some(out) = out
                        && tx.send(out).is_err()
                    {
                        break;
                    }
                }
            })
            .map_err(|e| crate::Error::InvalidOperation(format!("Failed to start watcher: {e}")))?;
        Ok(WorkspaceMonitor { rx })
    }
}

pub struct WorkspaceMonitor {
    rx: Receiver<WatchEvent>,
}

impl WorkspaceMonitor {
    pub fn receiver(&self) -> &Receiver<WatchEvent> {
        &self.rx
    }

    pub fn next_event(&self) -> Option<WatchEvent> {
        self.rx.recv().ok()
    }
}

pub struct StreamHandle {
    streamer: SessionStreamer,
}

impl StreamHandle {
    pub(crate) fn new(streamer: SessionStreamer) -> Self {
        Self { streamer }
    }

    pub fn receiver(&self) -> &Receiver<WatchEvent> {
        self.streamer.receiver()
    }
}
