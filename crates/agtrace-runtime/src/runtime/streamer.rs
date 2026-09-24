//! Live stream of one session's events, built on the incremental tailer.
//!
//! Every file of the session has a [`FileCursor`]; a poll decodes only newly
//! appended lines. The per-file event lists are kept (with the upsert rule) so
//! that the assembled sessions of the legacy [`StreamEvent`] API can be rebuilt
//! without re-parsing any file. New files of the session (e.g. Claude subagent
//! transcripts created after attach) are adopted as they appear.

use crate::runtime::events::{StreamEvent, WorkspaceEvent};
use crate::tail::{FileCursor, TailOutcome, provider_for};
use crate::{Error, Result};
use agtrace_engine::{AgentSession, assemble_sessions};
use agtrace_index::Database;
use agtrace_providers::{DecodeOptions, Provider, ProviderAdapter};
use agtrace_types::AgentEvent;
use notify::{Event, EventKind, PollWatcher, RecursiveMode, Watcher};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;
use uuid::Uuid;

/// How often tracked files are polled for appended bytes.
const POLL_INTERVAL: Duration = Duration::from_millis(100);

/// One tracked file: its cursor plus every event decoded from it so far.
struct TrackedFile {
    cursor: FileCursor,
    events: Vec<AgentEvent>,
    /// Event id -> index in `events` (upsert rule: same id replaces).
    index: HashMap<Uuid, usize>,
}

impl TrackedFile {
    fn new(cursor: FileCursor) -> Self {
        Self {
            cursor,
            events: Vec::new(),
            index: HashMap::new(),
        }
    }

    fn upsert(&mut self, event: AgentEvent) {
        match self.index.get(&event.id) {
            Some(&i) => self.events[i] = event,
            None => {
                self.index.insert(event.id, self.events.len());
                self.events.push(event);
            }
        }
    }

    /// Poll the cursor and fold the result. Returns the events that are new to
    /// the consumer (after a reset: only those not delivered before).
    fn poll(&mut self) -> std::io::Result<Vec<AgentEvent>> {
        match self.cursor.poll()? {
            TailOutcome::Unchanged => Ok(Vec::new()),
            TailOutcome::Appended(events) => {
                for event in &events {
                    self.upsert(event.clone());
                }
                Ok(events)
            }
            TailOutcome::Reset(events) => {
                let previous = std::mem::take(&mut self.index);
                self.events.clear();
                for event in &events {
                    self.upsert(event.clone());
                }
                Ok(events
                    .into_iter()
                    .filter(|e| !previous.contains_key(&e.id))
                    .collect())
            }
        }
    }
}

struct StreamContext {
    adapter: Arc<ProviderAdapter>,
    provider: Arc<dyn Provider>,
    session_id: String,
    /// Files known to belong to this session. Grows dynamically as new
    /// files appear (e.g., subagent transcripts created mid-session).
    files: Vec<TrackedFile>,
    /// Files confirmed to belong to a different session (negative cache,
    /// avoids re-reading headers on every fs poll tick).
    foreign_files: HashSet<PathBuf>,
    /// Assembled sessions (main + child streams)
    sessions: Vec<AgentSession>,
}

impl StreamContext {
    fn new(adapter: Arc<ProviderAdapter>, session_id: String, paths: Vec<PathBuf>) -> Self {
        let provider = provider_for(adapter.provider.id());
        let mut ctx = Self {
            adapter,
            provider,
            session_id,
            files: Vec::new(),
            foreign_files: HashSet::new(),
            sessions: Vec::new(),
        };
        for path in paths {
            ctx.track(path);
        }
        ctx
    }

    fn track(&mut self, path: PathBuf) {
        let cursor = FileCursor::new(self.provider.clone(), path, DecodeOptions::default());
        self.files.push(TrackedFile::new(cursor));
    }

    /// Adopt `path` if it is a newly appeared file of this session.
    fn consider(&mut self, path: &Path) {
        if self.files.iter().any(|f| f.cursor.path() == path) || self.foreign_files.contains(path) {
            return;
        }
        if !self.adapter.discovery.probe(path).is_match() {
            // Not cached: an empty or partially written file may become
            // a valid session file on a later poll tick.
            return;
        }
        match self.adapter.discovery.extract_session_id(path) {
            Ok(id) if id == self.session_id => self.track(path.to_path_buf()),
            Ok(_) => {
                self.foreign_files.insert(path.to_path_buf());
            }
            // Header not readable yet (e.g., first line still being
            // written) - retry on the next event.
            Err(_) => {}
        }
    }

    /// Poll every tracked file. Returns the newly decoded events (merged across
    /// files by timestamp, file order preserved) and re-assembles the sessions
    /// when anything changed.
    fn poll_all(&mut self) -> (Vec<AgentEvent>, Vec<String>) {
        let mut per_file = Vec::with_capacity(self.files.len());
        let mut errors = Vec::new();
        for file in &mut self.files {
            match file.poll() {
                Ok(new) => per_file.push(new),
                // Deleted (or not yet recreated): nothing to read.
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => per_file.push(Vec::new()),
                Err(e) => {
                    errors.push(format!("{}: {}", file.cursor.path().display(), e));
                    per_file.push(Vec::new());
                }
            }
        }
        let new_events = merge_by_timestamp(per_file.iter().map(Vec::as_slice).collect());
        if !new_events.is_empty() {
            let all = merge_by_timestamp(self.files.iter().map(|f| f.events.as_slice()).collect());
            self.sessions = assemble_sessions(&all);
        }
        (new_events, errors)
    }
}

/// Merge per-file event lists into one list ordered by timestamp.
///
/// Within a file the order is the file order (never re-sorted: timestamps are
/// display-only and may be non-monotonic); across files the earliest head wins,
/// ties going to the earlier file.
fn merge_by_timestamp(lists: Vec<&[AgentEvent]>) -> Vec<AgentEvent> {
    let total = lists.iter().map(|l| l.len()).sum();
    let mut out = Vec::with_capacity(total);
    let mut heads = vec![0usize; lists.len()];
    while out.len() < total {
        let next = (0..lists.len())
            .filter(|&i| heads[i] < lists[i].len())
            .min_by_key(|&i| (lists[i][heads[i]].timestamp, i))
            .expect("remaining events");
        out.push(lists[next][heads[next]].clone());
        heads[next] += 1;
    }
    out
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

        let mut context = StreamContext::new(provider, session_id, session_files);

        // Initial attach = tail every file from offset 0.
        if !publish(&mut context, &tx_out) {
            return Err(Error::InvalidOperation(
                "Stream receiver closed".to_string(),
            ));
        }

        let tx_worker = tx_out.clone();
        let handle = std::thread::Builder::new()
            .name("session-streamer".to_string())
            .spawn(move || {
                loop {
                    match rx_fs.recv_timeout(POLL_INTERVAL) {
                        Ok(event) => {
                            adopt_new_files(&event, &mut context);
                            // Coalesce a burst of fs events into one poll.
                            while let Ok(event) = rx_fs.try_recv() {
                                adopt_new_files(&event, &mut context);
                            }
                        }
                        Err(RecvTimeoutError::Timeout) => {}
                        Err(RecvTimeoutError::Disconnected) => {
                            let _ =
                                tx_worker.send(WorkspaceEvent::Stream(StreamEvent::Disconnected {
                                    reason: "Stream ended".to_string(),
                                }));
                            break;
                        }
                    }
                    if !publish(&mut context, &tx_worker) {
                        break; // receiver dropped
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

/// Poll all files and send what changed. Returns false once the receiver is gone.
fn publish(context: &mut StreamContext, tx: &Sender<WorkspaceEvent>) -> bool {
    let (events, errors) = context.poll_all();
    for e in errors {
        if tx
            .send(WorkspaceEvent::Error(format!("Stream error: {}", e)))
            .is_err()
        {
            return false;
        }
    }
    if events.is_empty() {
        return true;
    }
    tx.send(WorkspaceEvent::Stream(StreamEvent::Events {
        events,
        sessions: context.sessions.clone(),
    }))
    .is_ok()
}

/// Track files of this session that appeared after attach. Content changes of
/// tracked files are picked up by the periodic poll, not by fs events.
fn adopt_new_files(event: &Event, context: &mut StreamContext) {
    // Create matters too: subagent transcripts (e.g., Claude's
    // {session_id}/subagents/agent-*.jsonl) are created after attach.
    if let EventKind::Create(_) | EventKind::Modify(_) = event.kind {
        for path in &event.paths {
            context.consider(path);
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use agtrace_types::{AgentId, EventOrigin, EventPayload, UserPayload};
    use chrono::{DateTime, TimeZone, Utc};

    fn ev(agent: &AgentId, line: u64, ts: DateTime<Utc>) -> AgentEvent {
        AgentEvent {
            id: Uuid::new_v4(),
            session_id: Uuid::nil(),
            agent: agent.clone(),
            parent_id: None,
            timestamp: ts,
            origin: EventOrigin::new(line, 0, 0),
            payload: EventPayload::User(UserPayload {
                text: format!("{line}"),
            }),
        }
    }

    #[test]
    fn merge_keeps_file_order_even_with_non_monotonic_timestamps() {
        let main = AgentId::claude_session("s");
        let sub = AgentId::claude_subagent("s", "a1");
        let t = |s| Utc.with_ymd_and_hms(2026, 9, 20, 10, 0, s).unwrap();
        // Main file: timestamps go backwards at line 1.
        let a = vec![ev(&main, 0, t(10)), ev(&main, 1, t(5)), ev(&main, 2, t(20))];
        let b = vec![ev(&sub, 0, t(7)), ev(&sub, 1, t(15))];
        let merged = merge_by_timestamp(vec![&a, &b]);
        assert_eq!(merged.len(), 5);
        let main_lines: Vec<u64> = merged
            .iter()
            .filter(|e| e.agent == main)
            .map(|e| e.origin.line)
            .collect();
        assert_eq!(main_lines, vec![0, 1, 2], "file order must be preserved");
        let sub_lines: Vec<u64> = merged
            .iter()
            .filter(|e| e.agent == sub)
            .map(|e| e.origin.line)
            .collect();
        assert_eq!(sub_lines, vec![0, 1]);
        // Interleaved by timestamp across files: sub@7 precedes main@10.
        assert_eq!(merged[0].agent, sub);
    }

    #[test]
    fn upsert_replaces_event_with_same_id() {
        let dir = tempfile::tempdir().unwrap();
        let cursor = FileCursor::new(
            provider_for(agtrace_providers::ProviderId::ClaudeCode),
            dir.path().join("x.jsonl"),
            DecodeOptions::default(),
        );
        let mut file = TrackedFile::new(cursor);
        let agent = AgentId::claude_session("s");
        let t = Utc.with_ymd_and_hms(2026, 9, 20, 10, 0, 0).unwrap();
        let first = ev(&agent, 0, t);
        let mut again = first.clone();
        again.origin.line = 7;
        file.upsert(first);
        file.upsert(ev(&agent, 1, t));
        file.upsert(again);
        assert_eq!(file.events.len(), 2);
        assert_eq!(file.events[0].origin.line, 7);
    }
}
