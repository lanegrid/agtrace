use crate::runtime::events::{DiscoveryEvent, WorkspaceEvent};
use anyhow::Result;
use notify::{Event, EventKind, PollWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::JoinHandle;
use std::time::Duration;

pub struct WorkspaceSupervisor {
    _watcher: PollWatcher,
    _handle: JoinHandle<()>,
    rx: Receiver<WorkspaceEvent>,
}

impl WorkspaceSupervisor {
    pub fn start(watch_paths: Vec<PathBuf>) -> Result<Self> {
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

        for path in &watch_paths {
            if path.exists() {
                watcher.watch(path, RecursiveMode::Recursive)?;
            }
        }

        let tx_worker = tx_out.clone();
        let handle = std::thread::Builder::new()
            .name("workspace-supervisor".to_string())
            .spawn(move || loop {
                match rx_fs.recv_timeout(Duration::from_secs(5)) {
                    Ok(event) => {
                        handle_fs_event(&event, &tx_worker);
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

fn handle_fs_event(event: &Event, tx: &Sender<WorkspaceEvent>) {
    match event.kind {
        EventKind::Create(_) | EventKind::Modify(_) => {
            for path in &event.paths {
                if let Some(file_name) = path.file_name() {
                    let _ = tx.send(WorkspaceEvent::Discovery(DiscoveryEvent::SessionUpdated {
                        session_id: file_name.to_string_lossy().to_string(),
                    }));
                }
            }
        }
        _ => {}
    }
}
