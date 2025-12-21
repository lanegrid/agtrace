use crate::runtime::events::{DiscoveryEvent, WorkspaceEvent};
use agtrace_providers::LogProvider;
use anyhow::Result;
use notify::{Event, EventKind, PollWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

pub struct WatchContext {
    pub provider_name: String,
    pub provider: Arc<dyn LogProvider>,
    pub root: PathBuf,
}

pub struct WorkspaceSupervisor {
    _watcher: PollWatcher,
    _handle: JoinHandle<()>,
    rx: Receiver<WorkspaceEvent>,
}

impl WorkspaceSupervisor {
    pub fn start(contexts: Vec<WatchContext>) -> Result<Self> {
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
        let handle = std::thread::Builder::new()
            .name("workspace-supervisor".to_string())
            .spawn(move || loop {
                match rx_fs.recv_timeout(Duration::from_secs(5)) {
                    Ok(event) => {
                        handle_fs_event(&event, &contexts, &tx_worker);
                    }
                    Err(_) => {
                        // Periodic tick
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

fn handle_fs_event(event: &Event, contexts: &[WatchContext], tx: &Sender<WorkspaceEvent>) {
    match event.kind {
        EventKind::Create(_) | EventKind::Modify(_) => {
            for path in &event.paths {
                // Find matching provider for this path
                if let Some(context) = find_provider_for_path(path, contexts) {
                    // Use provider to validate and extract session ID
                    if context.provider.can_handle(path) {
                        match context.provider.extract_session_id(path) {
                            Ok(session_id) => {
                                let _ = tx.send(WorkspaceEvent::Discovery(
                                    DiscoveryEvent::SessionUpdated {
                                        session_id,
                                        provider_name: context.provider_name.clone(),
                                    },
                                ));
                            }
                            Err(_) => {
                                // Ignore files that can't be parsed (e.g., .DS_Store)
                            }
                        }
                    }
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
