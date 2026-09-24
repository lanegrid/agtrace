//! Workspace watcher (design §4.2): one background thread with its own bounded
//! polling loop that turns agent log files and Claude side state into
//! [`WorkspaceEvent`]s for the pure workspace fold (`agtrace_engine::workspace`).
//!
//! - **Every 250 ms** every tracked agent file is `stat`ed; files that grew (or
//!   shrank, or still hold an incomplete line) are polled through their
//!   [`FileCursor`](crate::tail) and the newly decoded events are emitted.
//! - **Every 1 s (discovery tick)** the *bounded* discovery set is re-listed:
//!   - Claude: the project's dirs under `<claude home>/projects` (`*.jsonl`), the
//!     `subagents/` dirs of tracked sessions (`agent-*.jsonl` + `.meta.json`),
//!     `<claude home>/sessions/<pid>.json` (never the `*.key` files) and
//!     `<claude home>/teams/*/config.json` of teams relevant to tracked agents.
//!   - Codex: `<codex home>/sessions/<today>/` and `<yesterday>/` (local dates).
//!     Each new rollout's header (line 0) is read once.
//! - A new file is read with `Provider::read_header` (retried next tick while it is
//!   not an agent file yet), filtered by project and time window ([`WatchScope`]),
//!   then tracked and tailed from offset 0.
//!
//! Nothing here walks a whole provider tree except the one-off lookup of an old
//! Codex root for [`WatchScope::Root`].

mod liveness;
mod state;

pub use agtrace_engine::workspace::{ProcessStatus, SideStateUpdate, TeamMember, WorkspaceEvent};

use crate::{Error, Result};
use agtrace_types::AgentId;
use state::WatcherState;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime};

/// Which agents a watcher follows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchScope {
    /// Every root agent of the project (cwd under `root`) whose file was modified
    /// within `since`, or whose Claude process is alive in the registry, plus all
    /// of its descendants.
    Project { root: PathBuf, since: Duration },
    /// One root agent and its descendants (`watch --session <id>`).
    Root(AgentId),
}

impl WatchScope {
    /// Default activity window of [`WatchScope::Project`].
    pub const DEFAULT_SINCE: Duration = Duration::from_secs(2 * 60 * 60);

    /// Project scope with the default 2 h window.
    pub fn project(root: impl Into<PathBuf>) -> Self {
        WatchScope::Project {
            root: root.into(),
            since: Self::DEFAULT_SINCE,
        }
    }
}

/// Provider directories the watcher reads. `None` disables that part.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WatchRoots {
    /// `~/.claude` (sessions registry, teams).
    pub claude_home: Option<PathBuf>,
    /// `~/.claude/projects` (transcripts).
    pub claude_projects: Option<PathBuf>,
    /// `~/.codex/sessions` (rollouts, `YYYY/MM/DD` dirs).
    pub codex_sessions: Option<PathBuf>,
}

impl WatchRoots {
    /// Default provider homes (honoring `AGTRACE_CLAUDE_HOME` / `AGTRACE_CODEX_HOME`).
    pub fn from_env() -> Self {
        Self {
            claude_home: agtrace_core::claude_home(),
            claude_projects: agtrace_core::claude_projects_root(),
            codex_sessions: agtrace_core::codex_sessions_root(),
        }
    }

    /// Roots below explicit provider homes (`<claude home>/projects`, `<codex home>/sessions`).
    pub fn from_homes(claude_home: Option<PathBuf>, codex_home: Option<PathBuf>) -> Self {
        Self {
            claude_projects: claude_home.as_ref().map(|h| h.join("projects")),
            claude_home,
            codex_sessions: codex_home.map(|h| h.join("sessions")),
        }
    }
}

/// Timing of the watcher loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WatcherOptions {
    /// How often tracked files are `stat`ed and polled.
    pub poll_interval: Duration,
    /// How often the discovery set is re-listed.
    pub discovery_interval: Duration,
}

impl Default for WatcherOptions {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_millis(250),
            discovery_interval: Duration::from_secs(1),
        }
    }
}

enum Control {
    Rescan,
    Stop,
}

/// Background workspace watcher. Dropping it stops the thread.
pub struct WorkspaceWatcher {
    rx: Receiver<WorkspaceEvent>,
    control: Sender<Control>,
    handle: Option<JoinHandle<()>>,
}

impl WorkspaceWatcher {
    /// Start watching `scope`. The first discovery tick runs immediately on the
    /// watcher thread; its events (initial tails of every agent in scope) arrive on
    /// [`receiver`](Self::receiver).
    pub fn start(scope: WatchScope, roots: WatchRoots, options: WatcherOptions) -> Result<Self> {
        let (tx, rx) = channel();
        let (control, control_rx) = channel();
        let state = WatcherState::new(scope, roots);
        let handle = std::thread::Builder::new()
            .name("workspace-watcher".to_string())
            .spawn(move || run(state, tx, control_rx, options))
            .map_err(|e| Error::InvalidOperation(format!("Failed to start watcher: {e}")))?;
        Ok(Self {
            rx,
            control,
            handle: Some(handle),
        })
    }

    pub fn receiver(&self) -> &Receiver<WorkspaceEvent> {
        &self.rx
    }

    /// Run a discovery tick now (e.g. the TUI `r` key).
    pub fn rescan(&self) {
        let _ = self.control.send(Control::Rescan);
    }

    /// A cheap handle that can trigger rescans from another thread.
    pub fn rescan_handle(&self) -> RescanHandle {
        RescanHandle(self.control.clone())
    }
}

impl Drop for WorkspaceWatcher {
    fn drop(&mut self) {
        let _ = self.control.send(Control::Stop);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// Triggers discovery ticks of a running [`WorkspaceWatcher`].
#[derive(Clone)]
pub struct RescanHandle(Sender<Control>);

impl RescanHandle {
    pub fn rescan(&self) {
        let _ = self.0.send(Control::Rescan);
    }
}

fn run(
    mut state: WatcherState,
    tx: Sender<WorkspaceEvent>,
    control: Receiver<Control>,
    options: WatcherOptions,
) {
    let mut next_discovery = Instant::now();
    loop {
        let mut out = Vec::new();
        if Instant::now() >= next_discovery {
            out.extend(state.discovery_tick(SystemTime::now()));
            next_discovery = Instant::now() + options.discovery_interval;
        }
        out.extend(state.poll_tick());
        for event in out {
            if tx.send(event).is_err() {
                return; // receiver dropped
            }
        }
        match control.recv_timeout(options.poll_interval) {
            Ok(Control::Rescan) => next_discovery = Instant::now(),
            Ok(Control::Stop) | Err(RecvTimeoutError::Disconnected) => return,
            Err(RecvTimeoutError::Timeout) => {}
        }
    }
}

#[cfg(test)]
mod tests;
