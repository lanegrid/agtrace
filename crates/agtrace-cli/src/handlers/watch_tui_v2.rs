//! Watch Handler for TUI v2
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

use crate::presentation::v2::presenters::watch_tui::build_screen_view_model;
use crate::presentation::v2::renderers::tui::{RendererSignal, TuiEvent, TuiRenderer};
use agtrace_engine::AgentSession;
use agtrace_runtime::{AgTrace, DiscoveryEvent, SessionState, StreamEvent, WorkspaceEvent};

pub enum WatchTarget {
    Provider { name: String },
    Session { id: String },
}

/// Handler state that manages domain data
struct WatchHandler {
    /// Session state
    state: SessionState,
    /// Event buffer
    events: VecDeque<agtrace_types::AgentEvent>,
    /// Assembled session (for turn metrics)
    assembled_session: Option<AgentSession>,
    /// Max context window
    max_context: Option<u32>,
    /// Sender to TUI renderer
    tx: Sender<TuiEvent>,
}

impl WatchHandler {
    fn new(state: SessionState, tx: Sender<TuiEvent>) -> Self {
        Self {
            state,
            events: VecDeque::new(),
            assembled_session: None,
            max_context: None,
            tx,
        }
    }

    /// Handle new event
    #[allow(dead_code)]
    fn handle_event(&mut self, event: agtrace_types::AgentEvent) {
        // Add to buffer (keep last 100 events)
        const MAX_EVENTS: usize = 100;
        self.events.push_back(event.clone());
        if self.events.len() > MAX_EVENTS {
            self.events.pop_front();
        }

        // Update state (simplified - real implementation would update token counts, etc.)
        self.state.event_count += 1;
        self.state.last_activity = event.timestamp;

        // Rebuild ViewModel and send to renderer
        self.send_update();
    }

    /// Reset session state (clear all buffers and data)
    fn reset_session_state(&mut self, session_id: String, path: Option<std::path::PathBuf>) {
        self.state = SessionState::new(session_id, path, chrono::Utc::now());
        self.events.clear();
        self.assembled_session = None;
        self.send_update();
    }

    /// Handle assembled session update
    #[allow(dead_code)]
    fn handle_session_update(&mut self, session: AgentSession) {
        self.assembled_session = Some(session);
        self.send_update();
    }

    /// Send updated ViewModel to renderer
    fn send_update(&self) {
        // Same fallback logic as build_dashboard: try context_window_limit first, then model lookup
        let token_limits = agtrace_runtime::TokenLimits::new();
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
            self.assembled_session.as_ref(),
            max_context_for_metrics,
        );

        // Send to renderer (ignore errors if renderer has quit)
        let _ = self.tx.send(TuiEvent::Update(Box::new(screen_vm)));
    }

    /// Send error to renderer
    fn send_error(&self, msg: String) {
        let _ = self.tx.send(TuiEvent::Error(msg));
    }
}

/// Main entry point for TUI v2 watch
pub fn handle(workspace: &AgTrace, project_root: Option<&Path>, target: WatchTarget) -> Result<()> {
    // Create channels for bidirectional communication
    let (event_tx, event_rx) = mpsc::channel(); // Handler -> Renderer (events)
    let (signal_tx, signal_rx) = mpsc::channel(); // Renderer -> Handler (signals)

    // Spawn TUI renderer thread
    let tui_handle = thread::spawn(move || {
        let renderer = TuiRenderer::new().with_signal_sender(signal_tx);
        renderer.run(event_rx)
    });

    // Run handler in main thread
    let result = run_handler(workspace, project_root, target, event_tx, signal_rx);

    // Wait for TUI to finish
    if let Err(e) = tui_handle.join() {
        eprintln!("TUI thread panicked: {:?}", e);
    }

    result
}

/// Run the handler loop
fn run_handler(
    workspace: &AgTrace,
    project_root: Option<&Path>,
    target: WatchTarget,
    tx: Sender<TuiEvent>,
    signal_rx: Receiver<RendererSignal>,
) -> Result<()> {
    let _watch_service = workspace.watch_service();

    match target {
        WatchTarget::Provider { name } => {
            handle_provider_watch(workspace, project_root, &name, tx, signal_rx)
        }
        WatchTarget::Session { id } => handle_session_watch(workspace, &id, tx, signal_rx),
    }
}

/// Handle provider watch mode
fn handle_provider_watch(
    workspace: &AgTrace,
    project_root: Option<&Path>,
    provider_name: &str,
    tx: Sender<TuiEvent>,
    signal_rx: Receiver<RendererSignal>,
) -> Result<()> {
    use agtrace_providers::ScanContext;
    use agtrace_runtime::SessionFilter;
    use agtrace_types::project_hash_from_root;

    let _watch_service = workspace.watch_service();

    // Quick scan to find latest session
    let latest_session = {
        let current_project_root = project_root.map(|p| p.display().to_string());
        let project_hash = if let Some(root) = &current_project_root {
            project_hash_from_root(root)
        } else {
            "unknown".to_string()
        };

        let scan_context = ScanContext {
            project_hash,
            project_root: current_project_root,
        };

        // Lightweight scan (incremental by default)
        let _ = workspace.projects().scan(&scan_context, false, |_| {});

        // Get latest session from updated DB
        let filter = SessionFilter::new()
            .source(provider_name.to_string())
            .limit(1);
        workspace
            .sessions()
            .list(filter)
            .ok()
            .and_then(|sessions| sessions.into_iter().next())
    };

    let Some(session) = latest_session else {
        let msg = format!("No sessions found for provider '{}'", provider_name);
        tx.send(TuiEvent::Error(msg))?;
        return Ok(());
    };

    // Watch the latest session
    handle_session_watch(workspace, &session.id, tx, signal_rx)
}

/// Handle session watch mode
fn handle_session_watch(
    workspace: &AgTrace,
    session_id: &str,
    tx: Sender<TuiEvent>,
    signal_rx: Receiver<RendererSignal>,
) -> Result<()> {
    use std::time::Duration;

    let watch_service = workspace.watch_service();

    // Start watching
    let handle = watch_service.watch_session(session_id)?;
    let rx_stream = handle.receiver();

    // Initialize handler with initial state
    let initial_state = SessionState::new(session_id.to_string(), None, chrono::Utc::now());

    let mut handler = WatchHandler::new(initial_state, tx.clone());

    // Set default fallback (will be updated from actual events)
    handler.max_context = Some(200_000); // Default to Claude Code's limit

    // Event loop
    loop {
        // Check for quit signal from renderer (non-blocking)
        match signal_rx.try_recv() {
            Ok(RendererSignal::Quit) => {
                // User requested quit
                break;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                // No signal, continue
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                // Renderer disconnected unexpectedly
                break;
            }
        }

        match rx_stream.recv_timeout(Duration::from_millis(500)) {
            Ok(WorkspaceEvent::Stream(stream_event)) => match stream_event {
                StreamEvent::Attached { session_id, path } => {
                    // Session attached - reset all state and buffers
                    handler.reset_session_state(session_id, Some(path));
                }
                StreamEvent::Events { events, session } => {
                    // Batch process events to avoid spamming TUI with updates
                    const MAX_EVENTS: usize = 100;

                    for event in &events {
                        // Update state with event data
                        handler.state.last_activity = event.timestamp;
                        handler.state.event_count += 1;

                        // Extract state updates
                        let updates = agtrace_engine::extract_state_updates(event);
                        if updates.is_new_turn {
                            handler.state.turn_count += 1;
                        }
                        if let Some(usage) = updates.usage {
                            handler.state.current_usage = usage;
                        }
                        if let Some(model) = updates.model {
                            if handler.state.model.is_none() {
                                handler.state.model = Some(model);
                            }
                        }
                        if let Some(limit) = updates.context_window_limit {
                            handler.state.context_window_limit = Some(limit);
                        }

                        // Add to event buffer (without triggering update)
                        handler.events.push_back(event.clone());
                        if handler.events.len() > MAX_EVENTS {
                            handler.events.pop_front();
                        }
                    }

                    // Update assembled session if available
                    if let Some(session) = session {
                        // Extract context_window_limit from assembled session if available
                        // The session's turns contain token usage that should reflect the actual limit
                        // Use the limit from state updates, or infer from session data
                        if handler.state.context_window_limit.is_none() {
                            // Try to infer from first turn's usage or use default
                            // For now, trust the state's context_window_limit from events
                        }
                        handler.assembled_session = Some(session);
                    }

                    // Send single update after processing all events (batch update)
                    handler.send_update();
                }
                StreamEvent::Disconnected { reason } => {
                    // Stream disconnected
                    handler.send_error(reason);
                    break;
                }
            },
            Ok(WorkspaceEvent::Discovery(discovery_event)) => match discovery_event {
                DiscoveryEvent::NewSession { summary } => {
                    // New session detected - reset to new session
                    handler.reset_session_state(summary.id.clone(), None);
                }
                DiscoveryEvent::SessionUpdated { .. } => {
                    // Session updated
                }
                DiscoveryEvent::SessionRemoved { .. } => {
                    // Session removed
                }
            },
            Ok(WorkspaceEvent::Error(msg)) => {
                handler.send_error(msg);
                break;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // Timeout, continue
                continue;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                // Stream closed
                break;
            }
        }
    }

    Ok(())
}
