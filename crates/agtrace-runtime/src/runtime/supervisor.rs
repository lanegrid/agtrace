use crate::runtime::events::{DiscoveryEvent, WorkspaceEvent};
use agtrace_index::Database;
use agtrace_providers::ProviderAdapter;
use anyhow::Result;
use notify::{Event, EventKind, PollWatcher, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

pub struct WatchContext {
    pub provider_name: String,
    pub provider: Arc<ProviderAdapter>,
    pub root: PathBuf,
}

pub struct WorkspaceSupervisor {
    _watcher: PollWatcher,
    _handle: JoinHandle<()>,
    rx: Receiver<WorkspaceEvent>,
}

impl WorkspaceSupervisor {
    pub fn start(contexts: Vec<WatchContext>, db: Arc<Mutex<Database>>) -> Result<Self> {
        let (tx_out, rx_out) = channel();
        let (tx_fs, rx_fs) = channel();

        let config = notify::Config::default().with_poll_interval(Duration::from_millis(1000));

        let mut watcher = PollWatcher::new(
            move |res: Result<Event, _>| {
                if let Ok(event) = res {
                    let _ = tx_fs.send(event);
                }
            },
            config,
        )?;

        for context in &contexts {
            if context.root.exists() {
                watcher.watch(&context.root, RecursiveMode::Recursive)?;
            }
        }

        let tx_worker = tx_out.clone();
        let seen_sessions = Arc::new(Mutex::new(HashSet::new()));
        let handle = std::thread::Builder::new()
            .name("workspace-supervisor".to_string())
            .spawn(move || {
                loop {
                    match rx_fs.recv_timeout(Duration::from_secs(5)) {
                        Ok(event) => {
                            handle_fs_event(&event, &contexts, &db, &seen_sessions, &tx_worker);
                        }
                        Err(_) => {
                            // Periodic tick
                        }
                    }
                }
            })?;

        Ok(Self {
            _watcher: watcher,
            _handle: handle,
            rx: rx_out,
        })
    }

    pub fn receiver(&self) -> &Receiver<WorkspaceEvent> {
        &self.rx
    }
}

fn handle_fs_event(
    event: &Event,
    contexts: &[WatchContext],
    _db: &Arc<Mutex<Database>>,
    seen_sessions: &Arc<Mutex<HashSet<String>>>,
    tx: &Sender<WorkspaceEvent>,
) {
    match event.kind {
        EventKind::Create(_) | EventKind::Modify(_) => {
            for path in &event.paths {
                if let Some(context) = find_provider_for_path(path, contexts)
                    && context.provider.discovery.probe(path).is_match()
                    && let Ok(session_id) = context.provider.discovery.extract_session_id(path)
                {
                    let mut seen = seen_sessions.lock().unwrap();
                    let is_new = seen.insert(session_id.clone());

                    let _ = tx.send(WorkspaceEvent::Discovery(DiscoveryEvent::SessionUpdated {
                        session_id,
                        provider_name: context.provider_name.clone(),
                        is_new,
                    }));
                }
            }
        }
        _ => {}
    }
}

fn find_provider_for_path<'a>(
    path: &Path,
    contexts: &'a [WatchContext],
) -> Option<&'a WatchContext> {
    contexts
        .iter()
        .find(|context| path.starts_with(&context.root))
}
