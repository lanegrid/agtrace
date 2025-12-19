use agtrace_engine::{assemble_session, AgentSession};
use agtrace_providers::LogProvider;
use agtrace_types::{AgentEvent, EventPayload};
use anyhow::Result;
use notify::{Event, EventKind, PollWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone)]
pub enum WatchTarget {
    Session {
        session_id: String,
        initial_path: PathBuf,
    },
    Waiting {
        message: String,
    },
}

#[derive(Debug, Clone)]
struct FileState {
    last_event_count: usize,
}

struct EventHandlerState {
    current_session_id: Option<String>,
    file_states: HashMap<PathBuf, FileState>,
}

struct EventHandlerContext<'a> {
    log_root: &'a Path,
    tx: &'a Sender<WatchEvent>,
    provider: &'a Arc<dyn LogProvider>,
    project_root: Option<&'a Path>,
}

#[derive(Debug, Clone)]
pub struct SessionUpdate {
    pub session: Option<AgentSession>,
    pub new_events: Vec<AgentEvent>,
    pub orphaned_events: Vec<AgentEvent>,
    pub total_events: usize,
}

#[derive(Debug, Clone)]
pub enum WatchEvent {
    Attached {
        path: PathBuf,
        session_id: Option<String>,
    },
    Update(SessionUpdate),
    SessionRotated {
        old_path: PathBuf,
        new_path: PathBuf,
    },
    Error(String),
    Waiting {
        message: String,
    },
}

pub struct SessionWatcher {
    _watcher: PollWatcher,
    rx: Receiver<WatchEvent>,
}

impl SessionWatcher {
    pub fn new(
        log_root: PathBuf,
        provider: Arc<dyn LogProvider>,
        explicit_target: Option<String>,
        project_root: Option<PathBuf>,
    ) -> Result<Self> {
        let (tx_out, rx_out) = channel();
        let (tx_fs, rx_fs) = channel();

        let target = if let Some(id_or_path) = explicit_target {
            resolve_explicit_target(&log_root, &id_or_path, &provider)?
        } else {
            find_active_target(&log_root, &provider, project_root.as_deref())?
        };

        let mut state = EventHandlerState {
            current_session_id: None,
            file_states: HashMap::new(),
        };

        let watch_dir = match &target {
            WatchTarget::Session { initial_path, .. } => {
                initial_path.parent().unwrap_or(&log_root).to_path_buf()
            }
            WatchTarget::Waiting { .. } => log_root.clone(),
        };

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

        match target {
            WatchTarget::Session {
                session_id,
                initial_path,
            } => {
                state.current_session_id = Some(session_id.clone());

                let _ = tx_out.send(WatchEvent::Attached {
                    path: initial_path.clone(),
                    session_id: Some(session_id.clone()),
                });

                let ctx = EventHandlerContext {
                    log_root: &log_root,
                    tx: &tx_out,
                    provider: &provider,
                    project_root: project_root.as_deref(),
                };

                if let Ok((all_events, new_events, total_events)) =
                    process_session_files(&session_id, &mut state.file_states, &ctx)
                {
                    if !new_events.is_empty() {
                        let update = build_session_update(all_events, new_events, total_events);
                        let _ = tx_out.send(WatchEvent::Update(update));
                    }
                }
            }
            WatchTarget::Waiting { message } => {
                let _ = tx_out.send(WatchEvent::Waiting { message });
            }
        }

        let tx_worker = tx_out.clone();
        std::thread::Builder::new()
            .name("session-watcher-worker".to_string())
            .spawn(move || {
                let context = EventHandlerContext {
                    log_root: &log_root,
                    tx: &tx_worker,
                    provider: &provider,
                    project_root: project_root.as_deref(),
                };

                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    while let Ok(event) = rx_fs.recv() {
                        if let Err(e) = handle_fs_event(&event, &mut state, &context) {
                            let _ = tx_worker.send(WatchEvent::Error(format!(
                                "File system event handling error: {}",
                                e
                            )));
                        }
                    }
                }));

                if let Err(panic_err) = result {
                    let panic_msg = if let Some(s) = panic_err.downcast_ref::<&str>() {
                        s.to_string()
                    } else if let Some(s) = panic_err.downcast_ref::<String>() {
                        s.clone()
                    } else {
                        "Worker thread panicked with unknown error".to_string()
                    };
                    let _ = tx_worker.send(WatchEvent::Error(format!(
                        "FATAL: Worker thread panicked: {}",
                        panic_msg
                    )));
                }
            })?;

        Ok(Self {
            _watcher: watcher,
            rx: rx_out,
        })
    }

    pub fn receiver(&self) -> &Receiver<WatchEvent> {
        &self.rx
    }
}

fn handle_fs_event(
    event: &Event,
    state: &mut EventHandlerState,
    context: &EventHandlerContext,
) -> Result<()> {
    match event.kind {
        EventKind::Create(_) | EventKind::Modify(_) => {
            for path in &event.paths {
                if !context.provider.can_handle(path) {
                    continue;
                }

                if let Some(root) = context.project_root {
                    if !context.provider.belongs_to_project(path, root) {
                        continue;
                    }
                }

                let session_id = match extract_session_id_from_file(path, context.provider) {
                    Ok(id) => id,
                    Err(_) => continue,
                };

                let is_different_session = state
                    .current_session_id
                    .as_ref()
                    .map(|current| current != &session_id)
                    .unwrap_or(true);

                if is_different_session {
                    let old_path = state.current_session_id.as_ref().and_then(|old_id| {
                        context
                            .provider
                            .find_session_files(context.log_root, old_id)
                            .ok()
                            .and_then(|files| files.first().cloned())
                    });

                    if let Some(old_p) = old_path {
                        let _ = context.tx.send(WatchEvent::SessionRotated {
                            old_path: old_p,
                            new_path: path.clone(),
                        });
                    }

                    state.current_session_id = Some(session_id.clone());

                    let _ = context.tx.send(WatchEvent::Attached {
                        path: path.clone(),
                        session_id: Some(session_id.clone()),
                    });

                    state.file_states.clear();
                }

                if let Ok((all_events, new_events, total_events)) =
                    process_session_files(&session_id, &mut state.file_states, context)
                {
                    if !new_events.is_empty() {
                        let update = build_session_update(all_events, new_events, total_events);
                        let _ = context.tx.send(WatchEvent::Update(update));
                    }
                }

                break;
            }
        }
        _ => {}
    }

    Ok(())
}

fn load_and_detect_changes(
    path: &Path,
    last_event_count: usize,
    provider: &Arc<dyn LogProvider>,
) -> Result<(Vec<AgentEvent>, Vec<AgentEvent>)> {
    let context = agtrace_providers::ImportContext {
        project_root_override: None,
        session_id_prefix: None,
        all_projects: false,
    };

    let all_events = provider.normalize_file(path, &context)?;

    let new_events = all_events.iter().skip(last_event_count).cloned().collect();

    Ok((all_events, new_events))
}

fn build_session_update(
    all_events: Vec<AgentEvent>,
    new_events: Vec<AgentEvent>,
    total_events: usize,
) -> SessionUpdate {
    let session = assemble_session(&all_events);

    let start_idx = all_events
        .iter()
        .position(|e| matches!(e.payload, EventPayload::User(_)))
        .unwrap_or(all_events.len());

    let orphaned_events = all_events.iter().take(start_idx).cloned().collect();

    SessionUpdate {
        session,
        new_events,
        orphaned_events,
        total_events,
    }
}

fn process_session_files(
    session_id: &str,
    file_states: &mut HashMap<PathBuf, FileState>,
    context: &EventHandlerContext,
) -> Result<(Vec<AgentEvent>, Vec<AgentEvent>, usize)> {
    let session_files = context
        .provider
        .find_session_files(context.log_root, session_id)?;

    let mut all_events_merged = Vec::new();
    let mut all_new_events = Vec::new();
    let mut total_events = 0;

    for file_path in session_files {
        let last_event_count = file_states
            .get(&file_path)
            .map(|s| s.last_event_count)
            .unwrap_or(0);

        if let Ok((all_events, new_events)) =
            load_and_detect_changes(&file_path, last_event_count, context.provider)
        {
            file_states.insert(
                file_path.clone(),
                FileState {
                    last_event_count: all_events.len(),
                },
            );

            all_events_merged.extend(all_events);
            all_new_events.extend(new_events);
            total_events += file_states
                .get(&file_path)
                .map(|s| s.last_event_count)
                .unwrap_or(0);
        }
    }

    all_events_merged.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    all_new_events.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

    Ok((all_events_merged, all_new_events, total_events))
}

fn resolve_explicit_target(
    log_root: &Path,
    id_or_path: &str,
    provider: &Arc<dyn LogProvider>,
) -> Result<WatchTarget> {
    let path_buf = PathBuf::from(id_or_path);

    if path_buf.exists() && path_buf.is_file() && provider.can_handle(&path_buf) {
        let session_id = extract_session_id_from_file(&path_buf, provider)?;
        return Ok(WatchTarget::Session {
            session_id,
            initial_path: path_buf,
        });
    }

    for entry in walkdir::WalkDir::new(log_root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        if path.is_file() && provider.can_handle(path) {
            if let Some(stem) = path.file_stem() {
                if stem.to_string_lossy().contains(id_or_path) {
                    let session_id = extract_session_id_from_file(path, provider)?;
                    return Ok(WatchTarget::Session {
                        session_id,
                        initial_path: path.to_path_buf(),
                    });
                }
            }
        }
    }

    anyhow::bail!(
        "No session file found for '{}'. Tried as file path and session ID.",
        id_or_path
    )
}

fn find_active_target(
    dir: &Path,
    provider: &Arc<dyn LogProvider>,
    project_root: Option<&Path>,
) -> Result<WatchTarget> {
    if !dir.exists() {
        return Ok(WatchTarget::Waiting {
            message: format!("Directory does not exist: {}", dir.display()),
        });
    }

    let now = SystemTime::now();
    let hot_threshold = Duration::from_secs(300);

    let mut entries: Vec<(PathBuf, SystemTime, u64)> = Vec::new();

    for entry in walkdir::WalkDir::new(dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        if !provider.can_handle(path) {
            continue;
        }

        if path.is_file() {
            if let Some(root) = project_root {
                if !provider.belongs_to_project(path, root) {
                    continue;
                }
            }

            if let Ok(metadata) = path.metadata() {
                if let Ok(modified) = metadata.modified() {
                    let size = metadata.len();
                    entries.push((path.to_path_buf(), modified, size));
                }
            }
        }
    }

    if entries.is_empty() {
        return Ok(WatchTarget::Waiting {
            message: "No session files found. Waiting for new session...".to_string(),
        });
    }

    entries.sort_by(|a, b| b.1.cmp(&a.1));

    let hot_sessions: Vec<_> = entries
        .iter()
        .filter(|(_, mtime, _)| {
            now.duration_since(*mtime)
                .map(|d| d < hot_threshold)
                .unwrap_or(false)
        })
        .collect();

    if hot_sessions.is_empty() {
        let (_path, latest_time, _) = &entries[0];
        let elapsed = now
            .duration_since(*latest_time)
            .unwrap_or(Duration::from_secs(0));

        let time_ago = format_duration(elapsed);
        return Ok(WatchTarget::Waiting {
            message: format!(
                "No active sessions found (last activity: {}). Waiting for new session...",
                time_ago
            ),
        });
    }

    let (path, _mtime, _size) = hot_sessions[0];

    let session_id = provider.extract_session_id(path)?;
    Ok(WatchTarget::Session {
        session_id,
        initial_path: path.clone(),
    })
}

fn format_duration(d: Duration) -> String {
    let secs = d.as_secs();
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else if secs < 86400 {
        format!("{}h", secs / 3600)
    } else {
        format!("{}d", secs / 86400)
    }
}

fn extract_session_id_from_file(path: &Path, provider: &Arc<dyn LogProvider>) -> Result<String> {
    provider.extract_session_id(path)
}
