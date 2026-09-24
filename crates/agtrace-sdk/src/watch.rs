//! Live multi-agent workspace (design §4.4).
//!
//! [`crate::Client::watch_workspace`] starts the runtime workspace watcher (bounded
//! polling of the project's agent files and Claude side state) and folds its
//! [`WorkspaceEvent`]s into a [`WorkspaceView`] on a background thread. Consumers
//! read a consistent snapshot of the view through [`LiveWorkspace::view`] /
//! [`LiveWorkspace::with_view`] and redraw when the generation counter changes
//! ([`LiveWorkspace::generation`], [`LiveWorkspace::subscribe`],
//! [`LiveWorkspace::changed`]).

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::RecvTimeoutError;
use std::sync::{Arc, RwLock, RwLockReadGuard};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use agtrace_engine::workspace::CatalogResolver;
use agtrace_runtime::{RescanHandle, WorkspaceWatcher};
use agtrace_types::ModelCatalog;

pub use agtrace_engine::workspace::{
    AgentStatus, AgentView, FeedEntry, StatusSource, TimelineEntry, TimelineItem, WorkspaceView,
};
pub use agtrace_runtime::{
    ProcessStatus, SideStateUpdate, TeamMember, WatchRoots, WatchScope, WatcherOptions,
    WorkspaceEvent,
};

/// How often time-based status rules (staleness) are re-evaluated.
const TICK_EVERY: Duration = Duration::from_secs(1);
/// Upper bound of events folded under one write lock (keeps readers responsive).
const MAX_BATCH: usize = 2048;

struct Shared {
    view: RwLock<WorkspaceView>,
    generation: AtomicU64,
}

/// A continuously updated [`WorkspaceView`]. Dropping it stops the watcher.
pub struct LiveWorkspace {
    shared: Arc<Shared>,
    gen_rx: tokio::sync::watch::Receiver<u64>,
    rescan: RescanHandle,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl LiveWorkspace {
    /// Fold the events of `watcher` into a fresh view on a background thread,
    /// resolving context windows against `catalog`.
    pub(crate) fn start(watcher: WorkspaceWatcher, catalog: Arc<dyn ModelCatalog>) -> Self {
        let shared = Arc::new(Shared {
            view: RwLock::new(WorkspaceView::new()),
            generation: AtomicU64::new(0),
        });
        let (gen_tx, gen_rx) = tokio::sync::watch::channel(0u64);
        let stop = Arc::new(AtomicBool::new(false));
        let rescan = watcher.rescan_handle();
        let handle = {
            let shared = shared.clone();
            let stop = stop.clone();
            std::thread::Builder::new()
                .name("workspace-fold".to_string())
                .spawn(move || fold_loop(watcher, catalog, shared, gen_tx, stop))
                .ok()
        };
        Self {
            shared,
            gen_rx,
            rescan,
            stop,
            handle,
        }
    }

    /// Read access to the current view (holds a read lock; keep it short).
    pub fn view(&self) -> RwLockReadGuard<'_, WorkspaceView> {
        self.shared
            .view
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Run `f` against the current view.
    pub fn with_view<R>(&self, f: impl FnOnce(&WorkspaceView) -> R) -> R {
        f(&self.view())
    }

    /// Bumped every time the view changed (new events, side state, status tick).
    pub fn generation(&self) -> u64 {
        self.shared.generation.load(Ordering::Acquire)
    }

    /// A receiver of generation changes (for async redraw loops).
    pub fn subscribe(&self) -> tokio::sync::watch::Receiver<u64> {
        self.gen_rx.clone()
    }

    /// Wait for the next view change. Returns `None` once the watcher stopped.
    pub async fn changed(&mut self) -> Option<u64> {
        self.gen_rx.changed().await.ok()?;
        Some(*self.gen_rx.borrow_and_update())
    }

    /// Block until `pred` holds for the view or `timeout` elapses (polls on
    /// generation changes). Returns whether `pred` held.
    pub fn wait_until(
        &self,
        timeout: Duration,
        mut pred: impl FnMut(&WorkspaceView) -> bool,
    ) -> bool {
        let deadline = Instant::now() + timeout;
        let mut seen = u64::MAX;
        loop {
            let generation = self.generation();
            if generation != seen {
                seen = generation;
                if self.with_view(&mut pred) {
                    return true;
                }
            }
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// Re-list the discovery set now (e.g. the TUI `r` key).
    pub fn rescan(&self) {
        self.rescan.rescan();
    }
}

impl Drop for LiveWorkspace {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn fold_loop(
    watcher: WorkspaceWatcher,
    catalog: Arc<dyn ModelCatalog>,
    shared: Arc<Shared>,
    gen_tx: tokio::sync::watch::Sender<u64>,
    stop: Arc<AtomicBool>,
) {
    let resolver = CatalogResolver(catalog.as_ref());
    let mut last_tick = Instant::now();
    while !stop.load(Ordering::Acquire) {
        let mut changed = false;
        match watcher.receiver().recv_timeout(Duration::from_millis(200)) {
            Ok(first) => {
                let mut view = shared
                    .view
                    .write()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                let now = chrono::Utc::now();
                view.apply(first, &resolver, now);
                for _ in 1..MAX_BATCH {
                    match watcher.receiver().try_recv() {
                        Ok(event) => view.apply(event, &resolver, now),
                        Err(_) => break,
                    }
                }
                changed = true;
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
        if last_tick.elapsed() >= TICK_EVERY {
            last_tick = Instant::now();
            let mut view = shared
                .view
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            changed |= view.tick(chrono::Utc::now());
        }
        if changed {
            let generation = shared.generation.fetch_add(1, Ordering::AcqRel) + 1;
            gen_tx.send_replace(generation);
        }
    }
}
