//! Watch Handler for TUI
//!
//! This module implements the Handler (Controller) that:
//! - Owns state (SessionState, event buffer)
//! - Subscribes to engine event streams
//! - Calls Presenter to build ViewModels
//! - Sends ViewModels to Renderer via channel

use std::collections::VecDeque;
use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use anyhow::Result;

use crate::presentation::presenters::watch_tui::build_screen_view_model;
use crate::presentation::renderers::tui::{RendererSignal, TuiEvent, TuiRenderer};
use agtrace_sdk::Client;
use agtrace_sdk::types::AgentSession;
use agtrace_sdk::types::{DiscoveryEvent, SessionState, StreamEvent, WorkspaceEvent};
use agtrace_sdk::utils::{extract_state_updates, filter_display_events};

pub enum WatchTarget {
    Provider { name: String },
    Session { id: String },
}

/// Handler state that manages domain data
struct WatchHandler {
    /// Session state
    state: SessionState,
    /// Event buffer (all streams combined)
    events: VecDeque<agtrace_sdk::types::AgentEvent>,
    /// Assembled sessions (main + child streams)
    assembled_sessions: Vec<AgentSession>,
    /// Max context window
    max_context: Option<u32>,
    /// Notification message (for session switching, etc.)
    notification: Option<String>,
    /// Project root (CWD)
    project_root: Option<std::path::PathBuf>,
    /// Sender to TUI renderer
    tx: Sender<TuiEvent>,
}

impl WatchHandler {
    fn new(
        state: SessionState,
        project_root: Option<std::path::PathBuf>,
        tx: Sender<TuiEvent>,
    ) -> Self {
        Self {
            state,
            events: VecDeque::new(),
            assembled_sessions: Vec::new(),
            max_context: None,
            notification: None,
            project_root,
            tx,
        }
    }

    /// Reset session state (clear all buffers and data)
    fn reset_session_state(&mut self, session_id: String, log_path: Option<std::path::PathBuf>) {
        self.state = SessionState::new(
            session_id,
            self.project_root.clone(),
            log_path,
            chrono::Utc::now(),
        );
        self.events.clear();
        self.assembled_sessions.clear();
        self.send_update();
    }

    /// Send updated ViewModel to renderer
    fn send_update(&self) {
        // Same fallback logic as build_dashboard: try context_window_limit first, then model lookup
        let token_limits = agtrace_sdk::utils::default_token_limits();
        let token_spec = self
            .state
            .model
            .as_ref()
            .and_then(|m| token_limits.get_limit(m));
        let limit_from_state_or_model = self
            .state
            .context_window_limit
            .or_else(|| token_spec.as_ref().map(|spec| spec.effective_limit()));

        // Fallback to handler's cached max_context if still None
        let max_context_for_metrics = limit_from_state_or_model
            .or(self.max_context.map(|c| c as u64))
            .map(|c| c as u32);

        // Call Presenter (pure function) to build ViewModel
        let screen_vm = build_screen_view_model(
            &self.state,
            &self.events,
            &self.assembled_sessions,
            max_context_for_metrics,
            self.notification.as_deref(),
        );

        // Send to renderer (ignore errors if renderer has quit)
        let _ = self.tx.send(TuiEvent::Update(Box::new(screen_vm)));
    }

    /// Send error to renderer
    fn send_error(&self, msg: String) {
        let _ = self.tx.send(TuiEvent::Error(msg));
    }

    /// Process stream events and update state
    ///
    /// Returns true if events were processed successfully, false for errors/disconnects
    fn process_stream_events(
        &mut self,
        events: Vec<agtrace_sdk::types::AgentEvent>,
        sessions: Vec<AgentSession>,
    ) {
        const MAX_EVENTS: usize = 100;

        // Store events for timeline display (recent events only)
        for event in &events {
            self.events.push_back(event.clone());
            if self.events.len() > MAX_EVENTS {
                self.events.pop_front();
            }
        }

        // Update state from display events (main stream only)
        let display_events = filter_display_events(&events);
        for event in &display_events {
            // Defensive check: only count main stream events
            // (filter_display_events should already filter, but verify to be safe)
            if !matches!(event.stream_id, agtrace_sdk::types::StreamId::Main) {
                continue;
            }

            self.state.last_activity = event.timestamp;
            self.state.event_count += 1;

            let updates = extract_state_updates(event);
            if updates.is_new_turn {
                self.state.turn_count += 1;
            }
            if let Some(usage) = updates.usage {
                self.state.current_usage = usage;
            }
            // Always update to latest model (not first-wins)
            if let Some(model) = updates.model {
                self.state.model = Some(model);
            }
            if let Some(limit) = updates.context_window_limit {
                self.state.context_window_limit = Some(limit);
            }
        }

        // Use sessions assembled by runtime (no local assembly needed)
        self.assembled_sessions = sessions;
    }
}

/// Main entry point for TUI watch
pub fn handle(client: &Client, project_root: Option<&Path>, target: WatchTarget) -> Result<()> {
    // Create channels for bidirectional communication
    let (event_tx, event_rx) = mpsc::channel(); // Handler -> Renderer (events)
    let (signal_tx, signal_rx) = mpsc::channel(); // Renderer -> Handler (signals)

    // Spawn TUI renderer thread
    let tui_handle = thread::spawn(move || {
        let renderer = TuiRenderer::new().with_signal_sender(signal_tx);
        renderer.run(event_rx)
    });

    // Run handler in main thread
    let result = run_handler(client, project_root, target, event_tx, signal_rx);

    // Wait for TUI to finish
    if let Err(e) = tui_handle.join() {
        eprintln!("TUI thread panicked: {:?}", e);
    }

    result
}

/// Run the handler loop
fn run_handler(
    client: &Client,
    project_root: Option<&Path>,
    target: WatchTarget,
    tx: Sender<TuiEvent>,
    signal_rx: Receiver<RendererSignal>,
) -> Result<()> {
    let _watch_service = client.watch_service();

    match target {
        WatchTarget::Provider { name } => {
            handle_provider_watch(client, project_root, &name, tx, signal_rx)
        }
        WatchTarget::Session { id } => handle_session_watch(client, &id, tx, signal_rx),
    }
}

/// Handle provider watch mode
fn handle_provider_watch(
    client: &Client,
    project_root: Option<&Path>,
    provider_name: &str,
    tx: Sender<TuiEvent>,
    signal_rx: Receiver<RendererSignal>,
) -> Result<()> {
    use agtrace_sdk::types::SessionFilter;
    use std::sync::mpsc::RecvTimeoutError;
    use std::time::Duration;

    let watch_service = client.watch_service();

    // Quick scan to find latest session
    let latest_session = {
        let scope = if let Some(root) = project_root {
            let hash = agtrace_sdk::utils::project_hash_from_root(&root.display().to_string());
            agtrace_sdk::types::ProjectScope::Specific(hash.clone())
        } else {
            agtrace_sdk::types::ProjectScope::All
        };

        // Lightweight scan (incremental by default)
        let _ = client
            .system()
            .reindex(scope.clone(), false, Some(provider_name), |_| {});

        // Get latest session from updated DB
        let filter = match scope {
            agtrace_sdk::types::ProjectScope::All => SessionFilter::all(),
            agtrace_sdk::types::ProjectScope::Specific(hash) => SessionFilter::project(hash),
        }
        .provider(provider_name.to_string())
        .limit(1);
        client
            .sessions()
            .list(filter)
            .ok()
            .and_then(|sessions| sessions.into_iter().next())
    };

    // NOTE: Watch all providers for cross-provider session switching
    // - Previously: only watched single provider (provider_name)
    // - Now: watch all enabled providers to detect new sessions from any provider
    // - This enables real-time switching to sessions from different providers
    let mut builder = watch_service.watch_all_providers()?;
    if let Some(root) = project_root {
        builder = builder.with_project_root(root.to_path_buf());
    }
    let monitor = builder.start_background_scan()?;
    let rx_discovery = monitor.receiver();

    // Initialize handler with initial session
    let initial_state = if let Some(session) = &latest_session {
        SessionState::new(session.id.clone(), None, None, chrono::Utc::now())
    } else {
        SessionState::new("waiting".to_string(), None, None, chrono::Utc::now())
    };

    let mut handler = WatchHandler::new(
        initial_state,
        project_root.map(|p| p.to_path_buf()),
        tx.clone(),
    );
    handler.max_context = Some(200_000); // Default fallback

    // Track current stream handle and mod_time for "most recently updated" switching
    let mut current_stream_handle: Option<agtrace_sdk::types::StreamHandle> = None;
    let mut current_session_id: Option<String> = None;
    let mut current_session_mod_time: Option<String> = None;

    // Attach to initial session if available
    if let Some(session) = latest_session {
        match watch_service.watch_session(&session.id) {
            Ok(handle) => {
                current_stream_handle = Some(handle);
                current_session_id = Some(session.id.clone());
                handler.notification = Some(format!("Watching session {}", &session.id[..8]));
                handler.send_update();
            }
            Err(e) => {
                handler.send_error(format!("Failed to attach to session: {}", e));
            }
        }
    } else {
        handler.notification = Some("Waiting for new session...".to_string());
        handler.send_update();
    }

    let poll_timeout = Duration::from_millis(100);

    // Event loop: monitor both discovery and stream events
    loop {
        // Check for quit signal from renderer
        match signal_rx.try_recv() {
            Ok(RendererSignal::Quit) => break,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
            _ => {}
        }

        // Check for new session discoveries (non-blocking)
        match rx_discovery.try_recv() {
            Ok(WorkspaceEvent::Discovery(DiscoveryEvent::NewSession { summary })) => {
                // Auto-switch to new session
                handler.notification = Some(format!("Switched to session {}", &summary.id[..8]));
                handler.reset_session_state(summary.id.clone(), None);

                match watch_service.watch_session(&summary.id) {
                    Ok(handle) => {
                        current_stream_handle = Some(handle);
                        current_session_id = Some(summary.id.clone());
                    }
                    Err(e) => {
                        handler.send_error(format!("Failed to attach to new session: {}", e));
                    }
                }
            }
            Ok(WorkspaceEvent::Discovery(DiscoveryEvent::SessionUpdated {
                session_id,
                is_new: _,
                mod_time,
                ..
            })) => {
                // NOTE: Switch to "most recently updated" session
                // - Old logic: only switch if is_new (first time seeing this session_id)
                // - New logic: switch if mod_time is newer than current session's mod_time
                // - This enables switching to existing sessions that get updated after startup
                let should_switch = if let Some(ref new_mod_time) = mod_time {
                    // Different session AND (no current mod_time OR newer mod_time)
                    current_session_id.as_ref() != Some(&session_id)
                        && (current_session_mod_time.is_none()
                            || Some(new_mod_time) > current_session_mod_time.as_ref())
                } else {
                    false
                };

                if should_switch {
                    handler.notification =
                        Some(format!("Switched to session {}", &session_id[..8]));
                    handler.reset_session_state(session_id.clone(), None);

                    match watch_service.watch_session(&session_id) {
                        Ok(handle) => {
                            current_stream_handle = Some(handle);
                            current_session_id = Some(session_id.clone());
                            current_session_mod_time = mod_time.clone();
                        }
                        Err(e) => {
                            handler
                                .send_error(format!("Failed to attach to updated session: {}", e));
                        }
                    }
                }
            }
            Ok(WorkspaceEvent::Error(msg)) => {
                if msg.starts_with("FATAL:") {
                    handler.send_error(msg);
                    break;
                }
            }
            _ => {}
        }

        // Process stream events from current session (drain all pending events)
        if let Some(ref stream_handle) = current_stream_handle {
            let rx = stream_handle.receiver();
            let mut received_events = false;
            let mut should_disconnect = false;

            // First, wait for at least one event (with timeout)
            match rx.recv_timeout(poll_timeout) {
                Ok(event) => {
                    if let Some(disconnect) =
                        process_workspace_event(event, &mut handler, &mut received_events)
                    {
                        should_disconnect = disconnect;
                    }
                }
                Err(RecvTimeoutError::Timeout) => {
                    // No events, continue
                }
                Err(RecvTimeoutError::Disconnected) => {
                    should_disconnect = true;
                }
            }

            // Drain all remaining pending events (non-blocking)
            if !should_disconnect {
                while let Ok(event) = rx.try_recv() {
                    if let Some(disconnect) =
                        process_workspace_event(event, &mut handler, &mut received_events)
                    {
                        should_disconnect = disconnect;
                        break;
                    }
                }
            }

            // Send single update after processing all events
            if received_events {
                handler.send_update();
            }

            if should_disconnect {
                current_stream_handle = None;
            }
        } else {
            // No active session, just wait
            std::thread::sleep(poll_timeout);
        }
    }

    Ok(())
}

/// Process a single workspace event
///
/// Returns Some(true) if should disconnect, Some(false) if should break loop,
/// None if should continue processing
fn process_workspace_event(
    event: WorkspaceEvent,
    handler: &mut WatchHandler,
    received_events: &mut bool,
) -> Option<bool> {
    match event {
        WorkspaceEvent::Stream(StreamEvent::Attached { session_id, path }) => {
            handler.reset_session_state(session_id, Some(path));
            None
        }
        WorkspaceEvent::Stream(StreamEvent::Events { events, sessions }) => {
            handler.process_stream_events(events, sessions);
            *received_events = true;
            None
        }
        WorkspaceEvent::Stream(StreamEvent::Disconnected { reason }) => {
            handler.notification = Some(format!(
                "Disconnected: {} - Waiting for new session...",
                reason
            ));
            handler.send_update();
            Some(true) // should disconnect
        }
        WorkspaceEvent::Error(msg) => {
            if msg.starts_with("FATAL:") {
                handler.send_error(msg);
                Some(false) // should break main loop (handled externally)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Handle session watch mode
fn handle_session_watch(
    client: &Client,
    session_id: &str,
    tx: Sender<TuiEvent>,
    signal_rx: Receiver<RendererSignal>,
) -> Result<()> {
    use std::sync::mpsc::RecvTimeoutError;
    use std::time::Duration;

    let watch_service = client.watch_service();

    // Start watching
    let handle = watch_service.watch_session(session_id)?;
    let rx_stream = handle.receiver();

    // Initialize handler with initial state
    let initial_state = SessionState::new(session_id.to_string(), None, None, chrono::Utc::now());

    let mut handler = WatchHandler::new(initial_state, None, tx.clone());

    // Set default fallback (will be updated from actual events)
    handler.max_context = Some(200_000); // Default to Claude Code's limit

    let poll_timeout = Duration::from_millis(100);

    // Event loop
    loop {
        // Check for quit signal from renderer (non-blocking)
        match signal_rx.try_recv() {
            Ok(RendererSignal::Quit) => break,
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
        }

        let mut received_events = false;
        let mut should_break = false;

        // First, wait for at least one event (with timeout)
        match rx_stream.recv_timeout(poll_timeout) {
            Ok(event) => {
                if process_workspace_event(event, &mut handler, &mut received_events) == Some(true)
                {
                    // For session watch, disconnect means end
                    should_break = true;
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                // No events, continue
            }
            Err(RecvTimeoutError::Disconnected) => {
                should_break = true;
            }
        }

        // Drain all remaining pending events (non-blocking)
        if !should_break {
            while let Ok(event) = rx_stream.try_recv() {
                if process_workspace_event(event, &mut handler, &mut received_events) == Some(true)
                {
                    should_break = true;
                    break;
                }
            }
        }

        // Send single update after processing all events
        if received_events {
            handler.send_update();
        }

        if should_break {
            break;
        }
    }

    Ok(())
}
