use crate::runtime::events::{DiscoveryEvent, WorkspaceEvent};
use crate::{Error, Result};
use agtrace_index::Database;
use agtrace_providers::ProviderAdapter;
use agtrace_types::project_hash_from_root;
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
    pub fn start(
        contexts: Vec<WatchContext>,
        db: Arc<Mutex<Database>>,
        project_root: Option<PathBuf>,
    ) -> Result<Self> {
        let (tx_out, rx_out) = channel();
        let (tx_fs, rx_fs) = channel();

        let config = notify::Config::default().with_poll_interval(Duration::from_millis(1000));

        let mut watcher = PollWatcher::new(
            move |res: std::result::Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    let _ = tx_fs.send(event);
                }
            },
            config,
        )
        .map_err(|e| Error::InvalidOperation(format!("Failed to create file watcher: {}", e)))?;

        for context in &contexts {
            if context.root.exists() {
                watcher
                    .watch(&context.root, RecursiveMode::Recursive)
                    .map_err(|e| {
                        Error::InvalidOperation(format!("Failed to watch directory: {}", e))
                    })?;
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
                            handle_fs_event(
                                &event,
                                &contexts,
                                &db,
                                &seen_sessions,
                                &tx_worker,
                                project_root.as_deref(),
                            );
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

// NOTE: Design rationale for mod_time in SessionUpdated
// - is_new flag only indicates "first time seeing this session_id in this process"
// - For "most recently updated" detection, we need actual file modification time
// - This enables watch mode to switch to actively updated sessions, even if they existed at startup
// - Without mod_time, watch would only switch to newly created sessions (is_new=true)
//
// NOTE: Project filtering
// - If project_root is Some, only emit events for sessions matching that project
// - If project_root is None, emit all events (backwards compatible, for --all-projects mode)
fn handle_fs_event(
    event: &Event,
    contexts: &[WatchContext],
    _db: &Arc<Mutex<Database>>,
    seen_sessions: &Arc<Mutex<HashSet<String>>>,
    tx: &Sender<WorkspaceEvent>,
    project_root: Option<&Path>,
) {
    match event.kind {
        EventKind::Create(_) | EventKind::Modify(_) => {
            for path in &event.paths {
                if let Some(context) = find_provider_for_path(path, contexts)
                    && context.provider.discovery.probe(path).is_match()
                    && let Ok(session_id) = context.provider.discovery.extract_session_id(path)
                {
                    // Project filtering: skip sessions from other projects
                    if let Some(filter_root) = project_root {
                        let filter_hash =
                            project_hash_from_root(&filter_root.display().to_string());

                        // Extract session's project hash from log file (lightweight operation)
                        if let Ok(Some(session_hash)) =
                            context.provider.discovery.extract_project_hash(path)
                        {
                            if filter_hash != session_hash {
                                // Session belongs to different project, skip it
                                continue;
                            }
                        } else {
                            // Cannot determine project, skip to be safe
                            continue;
                        }
                    }

                    let mut seen = seen_sessions.lock().unwrap();
                    let is_new = seen.insert(session_id.clone());

                    // Get file modification time for "most recently updated" detection (RFC3339 format)
                    let mod_time = std::fs::metadata(path)
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .map(|t| {
                            let dt: chrono::DateTime<chrono::Utc> = t.into();
                            dt.to_rfc3339()
                        });

                    let _ = tx.send(WorkspaceEvent::Discovery(DiscoveryEvent::SessionUpdated {
                        session_id,
                        provider_name: context.provider_name.clone(),
                        is_new,
                        mod_time,
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
