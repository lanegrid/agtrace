use crate::presentation::presenters;
use crate::presentation::renderers::traits::WatchView;
use crate::presentation::renderers::tui::TuiEvent;
use crate::presentation::renderers::TuiWatchView;
use crate::presentation::view_models::{WatchStart, WatchSummary};
use agtrace_runtime::{AgTrace, DiscoveryEvent, SessionState, StreamEvent, WorkspaceEvent};
use anyhow::Result;
use std::path::Path;
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError};
use std::time::Duration;

pub enum WatchTarget {
    Provider { name: String },
    Session { id: String },
}

pub fn handle(
    workspace: &AgTrace,
    project_root: Option<&Path>,
    target: WatchTarget,
    tui_view: TuiWatchView,
    rx: Receiver<TuiEvent>,
) -> Result<()> {
    use std::thread;

    let watch_service = workspace.watch_service();

    match target {
        WatchTarget::Provider { name } => {
            let log_root = watch_service
                .config()
                .providers
                .get(&name)
                .map(|p| p.log_root.clone())
                .unwrap_or_default();

            let start_event = WatchStart::Provider {
                name: name.clone(),
                log_root: log_root.clone(),
            };

            // Quick scan to ensure DB has latest sessions
            let latest_session = {
                use agtrace_providers::ScanContext;
                use agtrace_runtime::SessionFilter;
                use agtrace_types::project_hash_from_root;

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
                let filter = SessionFilter::new().source(name.clone()).limit(1);
                workspace
                    .sessions()
                    .list(filter)
                    .ok()
                    .and_then(|sessions| sessions.into_iter().next())
            };

            // Start workspace monitoring
            let mut builder = watch_service.watch_provider(&name)?;
            if let Some(root) = project_root {
                builder = builder.with_project_root(root.to_path_buf());
            }
            let monitor = builder.start_background_scan()?;

            // Create channel for discovery events
            let (tx_discovery, rx_discovery) = channel::<WorkspaceEvent>();

            // Monitor thread: forward discovery events
            thread::spawn(move || {
                while let Ok(event) = monitor.receiver().recv() {
                    let _ = tx_discovery.send(event);
                }
            });

            // Clone watch_service for thread (cheap Arc clone)
            let watch_service_clone = watch_service.clone();

            // Stream thread: attach to latest session or wait for new one
            thread::spawn(move || {
                let _ = tui_view.render_watch_start(&start_event);

                process_provider_events(
                    &watch_service_clone,
                    rx_discovery,
                    &tui_view,
                    latest_session,
                );
            });
        }
        WatchTarget::Session { id } => {
            let log_root = std::path::PathBuf::new();
            let start_event = WatchStart::Session {
                id: id.clone(),
                log_root,
            };

            // Attach to session
            let handle = watch_service.watch_session(&id)?;

            thread::spawn(move || {
                let _ = tui_view.render_watch_start(&start_event);
                process_stream_events(handle.receiver(), &tui_view, id);
            });
        }
    }

    // Run the TUI event loop on the main thread
    TuiWatchView::run(rx)
}

fn process_provider_events(
    watch_service: &agtrace_runtime::WatchService,
    rx_discovery: Receiver<WorkspaceEvent>,
    tui_view: &TuiWatchView,
    initial_session: Option<agtrace_index::SessionSummary>,
) {
    let mut current_handle: Option<agtrace_runtime::StreamHandle> = None;
    let mut current_session_id: Option<String> = None;
    let mut session_state: Option<SessionState> = None;
    let mut initialized = false;
    let poll_timeout = Duration::from_millis(100);

    // Attach to initial session if available
    if let Some(session) = initial_session {
        let _ = tui_view.on_watch_attached(&format!("Session {}", session.id));
        match watch_service.watch_session(&session.id) {
            Ok(handle) => {
                current_handle = Some(handle);
                current_session_id = Some(session.id.clone());
            }
            Err(e) => {
                let _ = tui_view.on_watch_error(&format!("Failed to attach: {}", e), false);
            }
        }
    } else {
        let _ = tui_view.on_watch_waiting("Waiting for new session...");
    }

    loop {
        // Check for new session discoveries (non-blocking)
        match rx_discovery.try_recv() {
            Ok(WorkspaceEvent::Discovery(DiscoveryEvent::NewSession { summary })) => {
                // TODO: Allow manual session selection via TUI
                // Currently auto-attaches to newly discovered sessions
                // Future: Show session list, allow user to select with keyboard
                // - Press 's' to toggle session list
                // - Press number to select session
                // - Maintain discovered_sessions: HashMap<String, SessionSummary>
                let _ = tui_view.on_watch_attached(&format!("Session {}", summary.id));

                // Create new session handle (replaces old one if exists)
                match watch_service.watch_session(&summary.id) {
                    Ok(handle) => {
                        current_handle = Some(handle);
                        current_session_id = Some(summary.id.clone());
                        session_state = None;
                        initialized = false;
                    }
                    Err(e) => {
                        let _ = tui_view.on_watch_error(&format!("Failed to attach: {}", e), false);
                    }
                }
            }
            Ok(WorkspaceEvent::Discovery(DiscoveryEvent::SessionUpdated {
                session_id,
                provider_name: _,
                is_new,
            })) => {
                // Switch to new session only if it's marked as new
                if is_new && current_session_id.as_ref() != Some(&session_id) {
                    let _ = tui_view.on_watch_attached(&format!("Session {}", session_id));

                    match watch_service.watch_session(&session_id) {
                        Ok(handle) => {
                            current_handle = Some(handle);
                            current_session_id = Some(session_id.clone());
                            session_state = None;
                            initialized = false;
                        }
                        Err(e) => {
                            let _ =
                                tui_view.on_watch_error(&format!("Failed to attach: {}", e), false);
                        }
                    }
                }
            }
            Ok(WorkspaceEvent::Error(msg)) => {
                let fatal = msg.starts_with("FATAL:");
                let _ = tui_view.on_watch_error(&msg, fatal);
                if fatal {
                    break;
                }
            }
            Ok(_) => {}  // Other discovery events
            Err(_) => {} // Channel empty or disconnected
        }

        // Process stream events from current session
        if let Some(ref handle) = current_handle {
            match handle.receiver().recv_timeout(poll_timeout) {
                Ok(WorkspaceEvent::Stream(StreamEvent::Attached { session_id, .. })) => {
                    let _ = tui_view.on_watch_attached(&session_id);
                }
                Ok(WorkspaceEvent::Stream(StreamEvent::Events { events, session })) => {
                    if session_state.is_none() && !events.is_empty() {
                        let session_id = current_session_id
                            .clone()
                            .unwrap_or_else(|| "unknown".to_string());

                        session_state =
                            Some(SessionState::new(session_id, None, events[0].timestamp));
                    }

                    if let Some(state) = &mut session_state {
                        for event in &events {
                            state.last_activity = event.timestamp;
                            state.event_count += 1;

                            let updates = agtrace_engine::extract_state_updates(event);
                            if updates.is_new_turn {
                                state.turn_count += 1;
                            }
                            if let Some(usage) = updates.usage {
                                state.current_usage = usage;
                            }
                            if let Some(model) = updates.model {
                                if state.model.is_none() {
                                    state.model = Some(model);
                                }
                            }
                        }

                        if !initialized {
                            let turn_count = session
                                .as_ref()
                                .map(|s| s.turns.len())
                                .unwrap_or(state.turn_count);

                            let _ = tui_view.on_watch_initial_summary(&WatchSummary {
                                recent_lines: Vec::new(),
                                token_usage: None,
                                turn_count,
                            });
                            initialized = true;
                        }

                        // Build turns data from assembled session
                        let turns_data = session.as_ref().map(|s| {
                            crate::presentation::renderers::tui::build_turns_from_session(s)
                        });

                        let event_vms = presenters::present_events(&events);
                        let state_vm = presenters::present_session_state(state);
                        let _ = tui_view.render_stream_update(
                            &state_vm,
                            &event_vms,
                            turns_data.as_deref(),
                        );
                    }
                }
                Ok(WorkspaceEvent::Stream(StreamEvent::Disconnected { reason })) => {
                    let _ = tui_view.on_watch_error(&reason, false);
                    current_handle = None;
                    let _ = tui_view.on_watch_waiting("Waiting for new session...");
                }
                Ok(WorkspaceEvent::Error(msg)) => {
                    let fatal = msg.starts_with("FATAL:");
                    let _ = tui_view.on_watch_error(&msg, fatal);
                    if fatal {
                        break;
                    }
                }
                Ok(_) => {}
                Err(RecvTimeoutError::Timeout) => {
                    // Continue to check for new sessions
                }
                Err(RecvTimeoutError::Disconnected) => {
                    current_handle = None;
                }
            }
        } else {
            // No active session, wait a bit before checking for new sessions
            std::thread::sleep(poll_timeout);
        }
    }
}

fn process_stream_events(
    receiver: &Receiver<WorkspaceEvent>,
    tui_view: &TuiWatchView,
    session_id: String,
) {
    let mut session_state: Option<SessionState> = None;
    let mut initialized = false;

    while let Ok(event) = receiver.recv() {
        match event {
            WorkspaceEvent::Stream(StreamEvent::Attached {
                session_id,
                path: _,
            }) => {
                let _ = tui_view.on_watch_attached(&session_id);
            }
            WorkspaceEvent::Stream(StreamEvent::Events { events, session }) => {
                // Initialize state on first events
                if session_state.is_none() && !events.is_empty() {
                    session_state = Some(SessionState::new(
                        session_id.clone(),
                        None,
                        events[0].timestamp,
                    ));
                }

                if let Some(state) = &mut session_state {
                    // Update state from events
                    for event in &events {
                        state.last_activity = event.timestamp;
                        state.event_count += 1;

                        let updates = agtrace_engine::extract_state_updates(event);
                        if updates.is_new_turn {
                            state.turn_count += 1;
                        }
                        if let Some(usage) = updates.usage {
                            state.current_usage = usage;
                        }
                        if let Some(model) = updates.model {
                            if state.model.is_none() {
                                state.model = Some(model);
                            }
                        }
                    }

                    // Show initial summary on first batch
                    if !initialized {
                        let turn_count = session
                            .as_ref()
                            .map(|s| s.turns.len())
                            .unwrap_or(state.turn_count);

                        let _ = tui_view.on_watch_initial_summary(&WatchSummary {
                            recent_lines: Vec::new(),
                            token_usage: None,
                            turn_count,
                        });
                        initialized = true;
                    }

                    // Build turns data from assembled session
                    let turns_data = session
                        .as_ref()
                        .map(crate::presentation::renderers::tui::build_turns_from_session);

                    let event_vms = presenters::present_events(&events);
                    let state_vm = presenters::present_session_state(state);
                    let _ =
                        tui_view.render_stream_update(&state_vm, &event_vms, turns_data.as_deref());
                }
            }
            WorkspaceEvent::Stream(StreamEvent::Disconnected { reason }) => {
                let _ = tui_view.on_watch_error(&reason, false);
                break;
            }
            WorkspaceEvent::Error(msg) => {
                let fatal = msg.starts_with("FATAL:");
                let _ = tui_view.on_watch_error(&msg, fatal);
                if fatal {
                    break;
                }
            }
            _ => {}
        }
    }
}
