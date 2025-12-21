use crate::runtime::events::{StreamEvent, WorkspaceEvent};
use agtrace_engine::{assemble_session, AgentSession};
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

struct StreamContext {
    provider: Arc<dyn LogProvider>,
    file_states: HashMap<PathBuf, usize>,
    all_events: Vec<AgentEvent>,
    session: Option<AgentSession>,
}

impl StreamContext {
    fn new(provider: Arc<dyn LogProvider>) -> Self {
        Self {
            provider,
            file_states: HashMap::new(),
            all_events: Vec::new(),
            session: None,
        }
    }

    fn load_all_events(&mut self, session_files: &[PathBuf]) -> Result<Vec<AgentEvent>> {
        let mut events = Vec::new();

        for path in session_files {
            let file_events = Self::load_file(path, &self.provider)?;
            self.file_states.insert(path.clone(), file_events.len());
            events.extend(file_events);
        }

        events.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        self.all_events = events.clone();
        self.session = assemble_session(&self.all_events);

        Ok(events)
    }

    fn handle_change(&mut self, path: &Path) -> Result<Vec<AgentEvent>> {
        let all_file_events = Self::load_file(path, &self.provider)?;
        let last_count = *self.file_states.get(path).unwrap_or(&0);

        if all_file_events.len() < last_count {
            self.file_states
                .insert(path.to_path_buf(), all_file_events.len());
            self.all_events = all_file_events.clone();
            self.session = assemble_session(&self.all_events);
            return Ok(all_file_events);
        }

        let new_events: Vec<AgentEvent> = all_file_events
            .into_iter()
            .skip(last_count)
            .collect();

        self.file_states
            .insert(path.to_path_buf(), last_count + new_events.len());

        self.all_events.extend(new_events.clone());
        self.all_events
            .sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        self.session = assemble_session(&self.all_events);

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

        Self::start_core(session_id, session_files, provider)
    }

    /// Attach to a session by scanning the filesystem for session files
    /// This is used when the session is not yet indexed in the database
    pub fn attach_from_filesystem(
        session_id: String,
        log_root: PathBuf,
        provider: Arc<dyn LogProvider>,
    ) -> Result<Self> {
        let session_files = find_session_files(&log_root, &session_id, &provider)?;

        if session_files.is_empty() {
            anyhow::bail!("No files found for session: {}", session_id);
        }

        Self::start_core(session_id, session_files, provider)
    }

    fn start_core(
        session_id: String,
        session_files: Vec<PathBuf>,
        provider: Arc<dyn LogProvider>,
    ) -> Result<Self> {
        let (tx_out, rx_out) = channel();
        let (tx_fs, rx_fs) = channel();

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

        let mut context = StreamContext::new(provider);

        if let Ok(events) = context.load_all_events(&session_files) {
            if !events.is_empty() {
                let _ = tx_out.send(WorkspaceEvent::Stream(StreamEvent::Events {
                    events: events.clone(),
                    session: context.session.clone(),
                }));
            }
        }

        let tx_worker = tx_out.clone();
        let handle = std::thread::Builder::new()
            .name("session-streamer".to_string())
            .spawn(move || loop {
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
    context: &mut StreamContext,
    tx: &Sender<WorkspaceEvent>,
) -> Result<()> {
    if let EventKind::Modify(_) = event.kind {
        for path in &event.paths {
            if session_files.contains(path) {
                if let Ok(new_events) = context.handle_change(path) {
                    if !new_events.is_empty() {
                        let _ = tx.send(WorkspaceEvent::Stream(StreamEvent::Events {
                            events: new_events,
                            session: context.session.clone(),
                        }));
                    }
                }
            }
        }
    }
    Ok(())
}

fn find_session_files(
    log_root: &Path,
    session_id: &str,
    provider: &Arc<dyn LogProvider>,
) -> Result<Vec<PathBuf>> {
    use std::fs;

    let mut session_files = Vec::new();

    fn visit_dir(
        dir: &Path,
        session_id: &str,
        provider: &Arc<dyn LogProvider>,
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
            } else if provider.can_handle(&path) {
                if let Ok(id) = provider.extract_session_id(&path) {
                    if id == session_id {
                        files.push(path);
                    }
                }
            }
        }

        Ok(())
    }

    visit_dir(log_root, session_id, provider, &mut session_files)?;
    Ok(session_files)
}
