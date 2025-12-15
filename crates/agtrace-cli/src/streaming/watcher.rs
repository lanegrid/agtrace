use agtrace_engine::{assemble_session_from_events, AgentSession};
use agtrace_providers::LogProvider;
use agtrace_types::v2::{AgentEvent, EventPayload};
use anyhow::Result;
use notify::{Event, EventKind, PollWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

/// Target for watch command - either an active file or waiting mode
#[derive(Debug, Clone)]
pub enum WatchTarget {
    /// Active file to attach to
    File { path: PathBuf },
    /// No active sessions - waiting mode
    Waiting { message: String },
}

/// Data payload for a session update
#[derive(Debug, Clone)]
pub struct SessionUpdate {
    /// The fully assembled session state (snapshot)
    pub session: Option<AgentSession>,
    /// New raw events detected in this update (delta)
    pub new_events: Vec<AgentEvent>,
    /// Events that were not included in the session (e.g. pre-session noise)
    #[allow(dead_code)]
    pub orphaned_events: Vec<AgentEvent>,
    /// Total count of events in the file
    #[allow(dead_code)]
    pub total_events: usize,
}

/// Events emitted by SessionWatcher
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// Successfully attached to a session file
    Attached {
        path: PathBuf,
        #[allow(dead_code)]
        session_id: Option<String>,
    },
    /// Session updated (new events and/or state change)
    Update(SessionUpdate),
    /// Session file was rotated (new session started)
    SessionRotated {
        #[allow(dead_code)]
        old_path: PathBuf,
        new_path: PathBuf,
    },
    /// Error occurred during watching or parsing
    Error(String),
    /// Waiting for new session (no active sessions found)
    Waiting { message: String },
}

/// Watches a directory for log files and streams events
pub struct SessionWatcher {
    _watcher: PollWatcher,
    rx: Receiver<StreamEvent>,
}

impl SessionWatcher {
    /// Create a new SessionWatcher that monitors the given log root directory
    /// If explicit_target is provided, it bypasses liveness detection and watches that specific file
    /// If project_root is provided, only sessions matching the project will be watched
    pub fn new(
        log_root: PathBuf,
        provider: Arc<dyn LogProvider>,
        explicit_target: Option<String>,
        project_root: Option<PathBuf>,
    ) -> Result<Self> {
        let (tx_out, rx_out) = channel();
        let (tx_fs, rx_fs) = channel();

        // Determine target: explicit or auto-detected
        let target = if let Some(id_or_path) = explicit_target {
            resolve_explicit_target(&log_root, &id_or_path, &provider)?
        } else {
            find_active_target(&log_root, &provider, project_root.as_deref())?
        };

        let mut current_file: Option<PathBuf> = None;
        let mut file_event_counts: HashMap<PathBuf, usize> = HashMap::new();

        // Determine which directory to watch based on target
        let watch_dir = match &target {
            WatchTarget::File { path } => {
                // Watch the parent directory of the file for better event detection
                // This is more reliable on macOS for deeply nested files
                path.parent().unwrap_or(&log_root).to_path_buf()
            }
            WatchTarget::Waiting { .. } => log_root.clone(),
        };

        // Set up polling-based watcher for better compatibility with macOS FSEvents
        // FSEvents coalesces rapid changes, causing delays in event delivery
        // Polling ensures we detect changes within 500ms regardless of FSEvents behavior
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
            WatchTarget::File { path } => {
                let event_count = match count_existing_events(&path, &provider) {
                    Ok(count) => count,
                    Err(e) => {
                        eprintln!("[WARN] Failed to count existing events: {}", e);
                        0
                    }
                };
                current_file = Some(path.clone());
                file_event_counts.insert(path.clone(), event_count);
                let _ = tx_out.send(StreamEvent::Attached {
                    path: path.clone(),
                    session_id: extract_session_id(&path),
                });
            }
            WatchTarget::Waiting { message } => {
                let _ = tx_out.send(StreamEvent::Waiting { message });
            }
        }

        // Spawn worker thread to handle file system events
        let tx_worker = tx_out.clone();
        std::thread::Builder::new()
            .name("session-watcher-worker".to_string())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    while let Ok(event) = rx_fs.recv() {
                        if let Err(e) = handle_fs_event(
                            &event,
                            &mut current_file,
                            &mut file_event_counts,
                            &tx_worker,
                            &provider,
                            project_root.as_deref(),
                        ) {
                            let _ = tx_worker.send(StreamEvent::Error(format!(
                                "File system event handling error: {}",
                                e
                            )));
                        }
                    }
                }));

                // Send error if worker panicked
                if let Err(panic_err) = result {
                    let panic_msg = if let Some(s) = panic_err.downcast_ref::<&str>() {
                        s.to_string()
                    } else if let Some(s) = panic_err.downcast_ref::<String>() {
                        s.clone()
                    } else {
                        "Worker thread panicked with unknown error".to_string()
                    };
                    let _ = tx_worker.send(StreamEvent::Error(format!(
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

    /// Get the receiver for stream events
    #[allow(dead_code)]
    pub fn receiver(&self) -> &Receiver<StreamEvent> {
        &self.rx
    }

    /// Consume self and return the receiver
    /// WARNING: This will drop the watcher, stopping file system monitoring.
    /// Only use this if you're managing the watcher lifetime externally.
    #[allow(dead_code)]
    pub fn into_receiver(self) -> Receiver<StreamEvent> {
        self.rx
    }
}

/// Handle a file system event
fn handle_fs_event(
    event: &Event,
    current_file: &mut Option<PathBuf>,
    file_event_counts: &mut HashMap<PathBuf, usize>,
    tx: &Sender<StreamEvent>,
    provider: &Arc<dyn LogProvider>,
    project_root: Option<&Path>,
) -> Result<()> {
    match event.kind {
        EventKind::Create(_) => {
            for path in &event.paths {
                // Use provider.can_handle() to respect provider-specific filtering
                if !provider.can_handle(path) {
                    continue;
                }

                // Check if file belongs to current project using provider
                if let Some(root) = project_root {
                    if !provider.belongs_to_project(path, root) {
                        continue;
                    }
                }

                // Check if this is a newer session than current
                let should_switch = if let Some(ref current) = current_file {
                    // Compare modification times
                    let new_time = std::fs::metadata(path)?.modified()?;
                    let current_time = std::fs::metadata(current)?.modified()?;
                    new_time > current_time
                } else {
                    true
                };

                if should_switch {
                    let old_path = current_file.clone();
                    *current_file = Some(path.clone());
                    // Count existing events when switching to new file
                    let event_count = match count_existing_events(path, provider) {
                        Ok(count) => count,
                        Err(e) => {
                            eprintln!("[WARN] Failed to count existing events: {}", e);
                            0
                        }
                    };
                    file_event_counts.insert(path.clone(), event_count);

                    if let Some(old) = old_path {
                        let _ = tx.send(StreamEvent::SessionRotated {
                            old_path: old,
                            new_path: path.clone(),
                        });
                    } else {
                        let _ = tx.send(StreamEvent::Attached {
                            path: path.clone(),
                            session_id: extract_session_id(path),
                        });
                    }
                }
            }
        }
        EventKind::Modify(_) => {
            for path in &event.paths {
                // If current_file is None, check if this is a new session file to attach to
                if current_file.is_none() && provider.can_handle(path) {
                    if let Some(root) = project_root {
                        if !provider.belongs_to_project(path, root) {
                            continue;
                        }
                    }

                    let event_count = match count_existing_events(path, provider) {
                        Ok(count) => count,
                        Err(e) => {
                            eprintln!("[WARN] Failed to count existing events: {}", e);
                            0
                        }
                    };
                    *current_file = Some(path.clone());
                    file_event_counts.insert(path.clone(), event_count);
                    let _ = tx.send(StreamEvent::Attached {
                        path: path.clone(),
                        session_id: extract_session_id(path),
                    });
                    continue;
                }

                if Some(path) == current_file.as_ref() {
                    let last_count = *file_event_counts.get(path).unwrap_or(&0);

                    if let Ok((all_events, new_events)) =
                        load_and_detect_changes(path, last_count, provider)
                    {
                        if !new_events.is_empty() {
                            let new_count = last_count + new_events.len();
                            file_event_counts.insert(path.clone(), new_count);

                            let session = assemble_session_from_events(&all_events);

                            let start_idx = all_events
                                .iter()
                                .position(|e| matches!(e.payload, EventPayload::User(_)))
                                .unwrap_or(all_events.len());

                            let orphaned_events =
                                all_events.iter().take(start_idx).cloned().collect();

                            let update = SessionUpdate {
                                session,
                                new_events,
                                orphaned_events,
                                total_events: all_events.len(),
                            };

                            let _ = tx.send(StreamEvent::Update(update));
                        }
                    }
                }
            }
        }
        _ => {}
    }

    Ok(())
}

/// Count existing events in a file by normalizing it
fn count_existing_events(path: &Path, provider: &Arc<dyn LogProvider>) -> Result<usize> {
    let context = agtrace_providers::ImportContext {
        project_root_override: None,
        session_id_prefix: None,
        all_projects: false,
    };
    let events = provider.normalize_file(path, &context)?;
    Ok(events.len())
}

/// Load full file and separate new events from old ones
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

/// Resolve an explicitly specified target (session ID or file path)
fn resolve_explicit_target(
    log_root: &Path,
    id_or_path: &str,
    provider: &Arc<dyn LogProvider>,
) -> Result<WatchTarget> {
    let path_buf = PathBuf::from(id_or_path);

    // Case 1: Direct file path (absolute or relative)
    if path_buf.exists() && path_buf.is_file() && provider.can_handle(&path_buf) {
        return Ok(WatchTarget::File { path: path_buf });
    }

    // Case 2: Session ID - search in log_root
    // Try to find a file matching the session ID pattern
    for entry in walkdir::WalkDir::new(log_root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        if path.is_file() && provider.can_handle(path) {
            if let Some(stem) = path.file_stem() {
                if stem.to_string_lossy().contains(id_or_path) {
                    return Ok(WatchTarget::File {
                        path: path.to_path_buf(),
                    });
                }
            }
        }
    }

    // Not found
    anyhow::bail!(
        "No session file found for '{}'. Tried as file path and session ID.",
        id_or_path
    )
}

/// Find an active target session using Liveness Window detection
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
    let hot_threshold = Duration::from_secs(300); // 5 minutes

    // Collect all log files with their metadata
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

    // Sort by modification time (newest first)
    entries.sort_by(|a, b| b.1.cmp(&a.1));

    // Find hot active sessions (< 5 min)
    let hot_sessions: Vec<_> = entries
        .iter()
        .filter(|(_, mtime, _)| {
            if let Ok(elapsed) = now.duration_since(*mtime) {
                elapsed < hot_threshold
            } else {
                false
            }
        })
        .collect();

    if hot_sessions.is_empty() {
        // All sessions are cold - enter waiting mode
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
    Ok(WatchTarget::File { path: path.clone() })
}

/// Format duration into human-readable string
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

/// Extract session ID from file path
fn extract_session_id(path: &Path) -> Option<String> {
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
}
