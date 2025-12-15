use agtrace_providers::LogProvider;
use agtrace_types::v2::AgentEvent;
use anyhow::Result;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

/// Target for watch command - either an active file or waiting mode
#[derive(Debug, Clone)]
pub enum WatchTarget {
    /// Active file to attach to
    File { path: PathBuf, offset: u64 },
    /// No active sessions - waiting mode
    Waiting { message: String },
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
    /// New events parsed from the log file
    NewEvents(Vec<AgentEvent>),
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
    _watcher: RecommendedWatcher,
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

        // Set up file system watcher
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, _>| {
            if let Ok(event) = res {
                let _ = tx_fs.send(event);
            }
        })?;

        watcher.watch(&log_root, RecursiveMode::Recursive)?;

        // Determine target: explicit or auto-detected
        let target = if let Some(id_or_path) = explicit_target {
            resolve_explicit_target(&log_root, &id_or_path)?
        } else {
            find_active_target(&log_root, &provider, project_root.as_deref())?
        };

        let mut current_file: Option<PathBuf> = None;
        let mut file_offsets: HashMap<PathBuf, u64> = HashMap::new();

        match target {
            WatchTarget::File { path, offset } => {
                current_file = Some(path.clone());
                file_offsets.insert(path.clone(), offset);
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
                // Catch panics to prevent silent failures
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    while let Ok(event) = rx_fs.recv() {
                        if let Err(e) = handle_fs_event(
                            &event,
                            &mut current_file,
                            &mut file_offsets,
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
    file_offsets: &mut HashMap<PathBuf, u64>,
    tx: &Sender<StreamEvent>,
    provider: &Arc<dyn LogProvider>,
    project_root: Option<&Path>,
) -> Result<()> {
    match event.kind {
        EventKind::Create(_) => {
            for path in &event.paths {
                if is_log_file(path) {
                    // Check if file belongs to current project using provider
                    if let Some(root) = project_root {
                        if !provider.belongs_to_project(path, root) {
                            // Ignore sessions from other projects
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
                        file_offsets.insert(path.clone(), 0);

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
        }
        EventKind::Modify(_) => {
            for path in &event.paths {
                if Some(path) == current_file.as_ref() {
                    // Read and parse new content
                    let offset = *file_offsets.get(path).unwrap_or(&0);
                    match process_new_lines(path, offset) {
                        Ok((new_offset, events)) => {
                            file_offsets.insert(path.clone(), new_offset);
                            if !events.is_empty() {
                                let _ = tx.send(StreamEvent::NewEvents(events));
                            }
                        }
                        Err(e) => {
                            let _ = tx.send(StreamEvent::Error(format!(
                                "Failed to read {}: {}",
                                path.display(),
                                e
                            )));
                        }
                    }
                }
            }
        }
        _ => {}
    }

    Ok(())
}

/// Process new lines from a file starting at the given offset
fn process_new_lines(path: &Path, offset: u64) -> Result<(u64, Vec<AgentEvent>)> {
    let mut file = File::open(path)?;
    file.seek(SeekFrom::Start(offset))?;
    let reader = BufReader::new(file);

    let mut new_offset = offset;
    let mut events = Vec::new();

    for line in reader.lines() {
        let line = line?;
        new_offset += line.len() as u64 + 1; // +1 for newline

        // Parse event (v2 schema)
        match serde_json::from_str::<AgentEvent>(&line) {
            Ok(event) => events.push(event),
            Err(_) => {
                // Skip malformed lines silently (could be incomplete writes)
            }
        }
    }

    Ok((new_offset, events))
}

/// Resolve an explicitly specified target (session ID or file path)
fn resolve_explicit_target(log_root: &Path, id_or_path: &str) -> Result<WatchTarget> {
    let path_buf = PathBuf::from(id_or_path);

    // Case 1: Direct file path (absolute or relative)
    if path_buf.exists() && path_buf.is_file() && is_log_file(&path_buf) {
        let metadata = std::fs::metadata(&path_buf)?;
        return Ok(WatchTarget::File {
            path: path_buf,
            offset: metadata.len(),
        });
    }

    // Case 2: Session ID - search in log_root
    // Try to find a file matching the session ID pattern
    for entry in walkdir::WalkDir::new(log_root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        if path.is_file() && is_log_file(path) {
            if let Some(stem) = path.file_stem() {
                if stem.to_string_lossy().contains(id_or_path) {
                    let metadata = std::fs::metadata(path)?;
                    return Ok(WatchTarget::File {
                        path: path.to_path_buf(),
                        offset: metadata.len(),
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
///
/// Priority order:
/// 1. Hot Active (< 5 min): Attach to the latest matching project
/// 2. Cold Dead (> 5 min): Enter waiting mode
/// 3. No files: Enter waiting mode
///
/// If project_root is provided, only considers files belonging to that project
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

        if path.is_file() && is_log_file(path) {
            // Filter by project if root is provided
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

    // We have at least one hot session
    let (path, _mtime, size) = hot_sessions[0];

    // Check for multiple hot sessions
    if hot_sessions.len() > 1 {
        eprintln!(
            "⚠️  Note: {} active sessions detected. Showing the latest one.",
            hot_sessions.len()
        );
    }

    Ok(WatchTarget::File {
        path: path.clone(),
        offset: *size,
    })
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

/// Check if a file is a log file (JSONL or JSON)
fn is_log_file(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .map(|ext| ext == "jsonl" || ext == "json")
        .unwrap_or(false)
}

/// Extract session ID from file path
fn extract_session_id(path: &Path) -> Option<String> {
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
}
