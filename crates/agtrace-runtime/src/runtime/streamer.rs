use crate::runtime::events::{StreamEvent, WorkspaceEvent};
use agtrace_index::Database;
use agtrace_providers::LogProvider;
use agtrace_types::AgentEvent;
use anyhow::Result;
use notify::{Event, EventKind, PollWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

struct FileState {
    last_event_count: usize,
}

pub struct SessionStreamer {
    _watcher: PollWatcher,
    _handle: JoinHandle<()>,
    rx: Receiver<WorkspaceEvent>,
}

impl SessionStreamer {
    pub fn receiver(&self) -> &Receiver<WorkspaceEvent> {
        &self.rx
    }

    pub fn attach(
        session_id: String,
        db: Arc<Mutex<Database>>,
        provider: Arc<dyn LogProvider>,
    ) -> Result<Self> {
        let (tx_out, rx_out) = channel();
        let (tx_fs, rx_fs) = channel();

        let session_files = {
            let db_lock = db.lock().unwrap();
            let files = db_lock.get_session_files(&session_id)?;
            if files.is_empty() {
                anyhow::bail!("Session not found: {}", session_id);
            }
            files
                .into_iter()
                .map(|f| PathBuf::from(f.path))
                .collect::<Vec<_>>()
        };

        let watch_dir = session_files
            .first()
            .and_then(|p| p.parent())
            .ok_or_else(|| anyhow::anyhow!("Cannot determine watch directory"))?
            .to_path_buf();

        let config = notify::Config::default().with_poll_interval(Duration::from_millis(500));

        let mut watcher = PollWatcher::new(
            move |res: Result<Event, _>| {
                if let Ok(event) = res {
                    let _ = tx_fs.send(event);
                }
            },
            config,
        )?;

        watcher.watch(&watch_dir, RecursiveMode::Recursive)?;

        let tx_attached = tx_out.clone();
        let first_file = session_files.first().cloned().unwrap();
        let _ = tx_attached.send(WorkspaceEvent::Stream(StreamEvent::Attached {
            session_id: session_id.clone(),
            path: first_file.clone(),
        }));

        let mut file_states: HashMap<PathBuf, FileState> = HashMap::new();

        if let Ok(events) = load_all_events(&session_files, &provider, &mut file_states) {
            if !events.is_empty() {
                let _ = tx_out.send(WorkspaceEvent::Stream(StreamEvent::Events {
                    events: events.clone(),
                }));
            }
        }

        let tx_worker = tx_out.clone();
        let handle = std::thread::Builder::new()
            .name("session-streamer".to_string())
            .spawn(move || loop {
                match rx_fs.recv() {
                    Ok(event) => {
                        if let Err(e) = handle_fs_event(
                            &event,
                            &session_files,
                            &provider,
                            &mut file_states,
                            &tx_worker,
                        ) {
                            let _ = tx_worker
                                .send(WorkspaceEvent::Error(format!("Stream error: {}", e)));
                        }
                    }
                    Err(_) => {
                        let _ = tx_worker.send(WorkspaceEvent::Stream(StreamEvent::Disconnected {
                            reason: "Stream ended".to_string(),
                        }));
                        break;
                    }
                }
            })?;

        Ok(Self {
            _watcher: watcher,
            _handle: handle,
            rx: rx_out,
        })
    }
}

fn handle_fs_event(
    event: &Event,
    session_files: &[PathBuf],
    provider: &Arc<dyn LogProvider>,
    file_states: &mut HashMap<PathBuf, FileState>,
    tx: &Sender<WorkspaceEvent>,
) -> Result<()> {
    if let EventKind::Modify(_) = event.kind {
        for path in &event.paths {
            if session_files.contains(path) {
                if let Ok(new_events) = load_new_events(path, provider, file_states) {
                    if !new_events.is_empty() {
                        let _ = tx.send(WorkspaceEvent::Stream(StreamEvent::Events {
                            events: new_events,
                        }));
                    }
                }
            }
        }
    }
    Ok(())
}

fn load_all_events(
    session_files: &[PathBuf],
    provider: &Arc<dyn LogProvider>,
    file_states: &mut HashMap<PathBuf, FileState>,
) -> Result<Vec<AgentEvent>> {
    let mut all_events = Vec::new();

    for path in session_files {
        let events = load_file(path, provider)?;
        file_states.insert(
            path.clone(),
            FileState {
                last_event_count: events.len(),
            },
        );
        all_events.extend(events);
    }

    all_events.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    Ok(all_events)
}

fn load_new_events(
    path: &Path,
    provider: &Arc<dyn LogProvider>,
    file_states: &mut HashMap<PathBuf, FileState>,
) -> Result<Vec<AgentEvent>> {
    let all_events = load_file(path, provider)?;
    let last_count = file_states
        .get(path)
        .map(|s| s.last_event_count)
        .unwrap_or(0);

    let new_events: Vec<AgentEvent> = all_events.iter().skip(last_count).cloned().collect();

    file_states.insert(
        path.to_path_buf(),
        FileState {
            last_event_count: all_events.len(),
        },
    );

    Ok(new_events)
}

fn load_file(path: &Path, provider: &Arc<dyn LogProvider>) -> Result<Vec<AgentEvent>> {
    let context = agtrace_providers::ImportContext {
        project_root_override: None,
        session_id_prefix: None,
        all_projects: false,
    };

    provider.normalize_file(path, &context)
}
