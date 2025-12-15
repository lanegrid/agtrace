use agtrace_types::v2::AgentEvent;
use anyhow::Result;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::SystemTime;

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
}

/// Watches a directory for log files and streams events
pub struct SessionWatcher {
    _watcher: RecommendedWatcher,
    rx: Receiver<StreamEvent>,
}

impl SessionWatcher {
    /// Create a new SessionWatcher that monitors the given log root directory
    pub fn new(log_root: PathBuf) -> Result<Self> {
        let (tx_out, rx_out) = channel();
        let (tx_fs, rx_fs) = channel();

        // Set up file system watcher
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, _>| {
            if let Ok(event) = res {
                let _ = tx_fs.send(event);
            }
        })?;

        watcher.watch(&log_root, RecursiveMode::Recursive)?;

        // Find and attach to the latest session file
        let initial_file = find_latest_log_file(&log_root)?;
        let mut current_file = initial_file.clone();
        let mut file_offsets: HashMap<PathBuf, u64> = HashMap::new();

        if let Some(ref path) = initial_file {
            let offset = std::fs::metadata(path)?.len();
            file_offsets.insert(path.clone(), offset);
            let _ = tx_out.send(StreamEvent::Attached {
                path: path.clone(),
                session_id: extract_session_id(path),
            });
        }

        // Spawn worker thread to handle file system events
        let tx_worker = tx_out.clone();
        std::thread::spawn(move || {
            while let Ok(event) = rx_fs.recv() {
                if let Err(e) =
                    handle_fs_event(&event, &mut current_file, &mut file_offsets, &tx_worker)
                {
                    let _ = tx_worker.send(StreamEvent::Error(e.to_string()));
                }
            }
        });

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
) -> Result<()> {
    match event.kind {
        EventKind::Create(_) => {
            for path in &event.paths {
                if is_log_file(path) {
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

/// Find the most recently modified log file in the directory (recursive)
fn find_latest_log_file(dir: &Path) -> Result<Option<PathBuf>> {
    if !dir.exists() {
        return Ok(None);
    }

    let mut latest: Option<(PathBuf, SystemTime)> = None;

    for entry in walkdir::WalkDir::new(dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        if path.is_file() && is_log_file(path) {
            if let Ok(metadata) = path.metadata() {
                if let Ok(modified) = metadata.modified() {
                    if let Some((_, latest_time)) = &latest {
                        if modified > *latest_time {
                            latest = Some((path.to_path_buf(), modified));
                        }
                    } else {
                        latest = Some((path.to_path_buf(), modified));
                    }
                }
            }
        }
    }

    Ok(latest.map(|(path, _)| path))
}

/// Check if a file is a log file (JSONL)
fn is_log_file(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .map(|ext| ext == "jsonl")
        .unwrap_or(false)
}

/// Extract session ID from file path
fn extract_session_id(path: &Path) -> Option<String> {
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
}
