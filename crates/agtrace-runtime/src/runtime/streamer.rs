use crate::runtime::events::{StreamEvent, WorkspaceEvent};
use crate::{Error, Result};
use agtrace_engine::{AgentSession, assemble_sessions};
use agtrace_index::Database;
use agtrace_providers::ProviderAdapter;
use agtrace_types::AgentEvent;
use notify::{Event, EventKind, PollWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

struct StreamContext {
    provider: Arc<ProviderAdapter>,
    /// Events per file, preserving file-internal order
    file_events: HashMap<PathBuf, Vec<AgentEvent>>,
    /// Assembled sessions (main + child streams)
    sessions: Vec<AgentSession>,
}

impl StreamContext {
    fn new(provider: Arc<ProviderAdapter>) -> Self {
        Self {
            provider,
            file_events: HashMap::new(),
            sessions: Vec::new(),
        }
    }

    fn load_all_events(&mut self, session_files: &[PathBuf]) -> Result<Vec<AgentEvent>> {
        for path in session_files {
            let events = Self::load_file(path, &self.provider)?;
            self.file_events.insert(path.clone(), events);
        }

        let all_events = self.merge_all_events();
        self.sessions = assemble_sessions(&all_events);

        Ok(all_events)
    }

    fn handle_change(&mut self, path: &Path) -> Result<Vec<AgentEvent>> {
        let all_file_events = Self::load_file(path, &self.provider)?;
        let last_count = self.file_events.get(path).map(|e| e.len()).unwrap_or(0);

        // Determine new events for the return value
        let new_events: Vec<AgentEvent> = if all_file_events.len() >= last_count {
            all_file_events.iter().skip(last_count).cloned().collect()
        } else {
            // File shrunk (e.g., log rotation) - treat all events as new
            all_file_events.clone()
        };

        // Replace the entire file's events to preserve correct ordering
        // This fixes the bug where extend + sort would break event ordering
        // for events with identical timestamps (e.g., ToolCall before ToolResult)
        self.file_events.insert(path.to_path_buf(), all_file_events);

        // Rebuild all_events from all files and reassemble sessions
        let all_events = self.merge_all_events();
        self.sessions = assemble_sessions(&all_events);

        Ok(new_events)
    }

    /// Merge events from all files, sorting by timestamp while preserving
    /// file-internal order for events with identical timestamps
    fn merge_all_events(&self) -> Vec<AgentEvent> {
        let mut all_events: Vec<AgentEvent> =
            self.file_events.values().flatten().cloned().collect();
        // Stable sort preserves file-internal order for same-timestamp events
        all_events.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        all_events
    }

    fn load_file(path: &Path, provider: &Arc<ProviderAdapter>) -> Result<Vec<AgentEvent>> {
        Ok(provider.parser.parse_file(path)?)
    }
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
        provider: Arc<ProviderAdapter>,
    ) -> Result<Self> {
        let session_files = {
            let db_lock = db.lock().unwrap();
            let files = db_lock.get_session_files(&session_id)?;
            if files.is_empty() {
                return Err(Error::InvalidOperation(format!(
                    "Session not found: {}",
                    session_id
                )));
            }
            files
                .into_iter()
                .map(|f| PathBuf::from(f.path))
                .collect::<Vec<_>>()
        };

        Self::start_core(session_id, session_files, provider)
    }

    /// Attach to a session by scanning the filesystem for session files
    /// This is used when the session is not yet indexed in the database
    pub fn attach_from_filesystem(
        session_id: String,
        log_root: PathBuf,
        provider: Arc<ProviderAdapter>,
    ) -> Result<Self> {
        let session_files = find_session_files(&log_root, &session_id, &provider)?;

        if session_files.is_empty() {
            return Err(Error::InvalidOperation(format!(
                "No files found for session: {}",
                session_id
            )));
        }

        Self::start_core(session_id, session_files, provider)
    }

    fn start_core(
        session_id: String,
        session_files: Vec<PathBuf>,
        provider: Arc<ProviderAdapter>,
    ) -> Result<Self> {
        let (tx_out, rx_out) = channel();
        let (tx_fs, rx_fs) = channel();

        let watch_dir = session_files
            .first()
            .and_then(|p| p.parent())
            .ok_or_else(|| Error::InvalidOperation("Cannot determine watch directory".to_string()))?
            .to_path_buf();

        let config = notify::Config::default().with_poll_interval(Duration::from_millis(100));

        let mut watcher = PollWatcher::new(
            move |res: std::result::Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    let _ = tx_fs.send(event);
                }
            },
            config,
        )
        .map_err(|e| Error::InvalidOperation(format!("Failed to create file watcher: {}", e)))?;

        watcher
            .watch(&watch_dir, RecursiveMode::Recursive)
            .map_err(|e| Error::InvalidOperation(format!("Failed to watch directory: {}", e)))?;

        let tx_attached = tx_out.clone();
        let first_file = session_files.first().cloned().unwrap();
        let _ = tx_attached.send(WorkspaceEvent::Stream(StreamEvent::Attached {
            session_id: session_id.clone(),
            path: first_file.clone(),
        }));

        let mut context = StreamContext::new(provider);

        if let Ok(events) = context.load_all_events(&session_files)
            && !events.is_empty()
        {
            let _ = tx_out.send(WorkspaceEvent::Stream(StreamEvent::Events {
                events: events.clone(),
                sessions: context.sessions.clone(),
            }));
        }

        let tx_worker = tx_out.clone();
        let handle = std::thread::Builder::new()
            .name("session-streamer".to_string())
            .spawn(move || {
                loop {
                    match rx_fs.recv() {
                        Ok(event) => {
                            if let Err(e) =
                                handle_fs_event(&event, &session_files, &mut context, &tx_worker)
                            {
                                let _ = tx_worker
                                    .send(WorkspaceEvent::Error(format!("Stream error: {}", e)));
                            }
                        }
                        Err(_) => {
                            let _ =
                                tx_worker.send(WorkspaceEvent::Stream(StreamEvent::Disconnected {
                                    reason: "Stream ended".to_string(),
                                }));
                            break;
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
}

fn handle_fs_event(
    event: &Event,
    session_files: &[PathBuf],
    context: &mut StreamContext,
    tx: &Sender<WorkspaceEvent>,
) -> Result<()> {
    if let EventKind::Modify(_) = event.kind {
        for path in &event.paths {
            if session_files.contains(path)
                && let Ok(new_events) = context.handle_change(path)
                && !new_events.is_empty()
            {
                let _ = tx.send(WorkspaceEvent::Stream(StreamEvent::Events {
                    events: new_events,
                    sessions: context.sessions.clone(),
                }));
            }
        }
    }
    Ok(())
}

fn find_session_files(
    log_root: &Path,
    session_id: &str,
    provider: &Arc<ProviderAdapter>,
) -> Result<Vec<PathBuf>> {
    use std::fs;

    let mut session_files = Vec::new();

    fn visit_dir(
        dir: &Path,
        session_id: &str,
        provider: &Arc<ProviderAdapter>,
        files: &mut Vec<PathBuf>,
    ) -> Result<()> {
        if !dir.is_dir() {
            return Ok(());
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                visit_dir(&path, session_id, provider, files)?;
            } else if provider.discovery.probe(&path).is_match()
                && let Ok(id) = provider.discovery.extract_session_id(&path)
                && id == session_id
            {
                files.push(path);
            }
        }

        Ok(())
    }

    visit_dir(log_root, session_id, provider, &mut session_files)?;
    Ok(session_files)
}
