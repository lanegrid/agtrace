use crate::presentation::presenters;
use crate::presentation::renderers::traits::WatchView;
use crate::presentation::renderers::tui::TuiEvent;
use crate::presentation::renderers::TuiWatchView;
use crate::presentation::view_models::{WatchStart, WatchSummary};
use agtrace_engine::assemble_session;
use agtrace_runtime::{
    AgTrace, DiscoveryEvent, SessionState, SessionStreamer, StreamEvent, WorkspaceEvent,
};
use anyhow::Result;
use std::path::Path;
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};

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

    match target {
        WatchTarget::Provider { name } => {
            let log_root = workspace
                .config()
                .providers
                .get(&name)
                .map(|p| p.log_root.clone())
                .unwrap_or_default();

            let start_event = WatchStart::Provider {
                name: name.clone(),
                log_root,
            };

            // Start workspace monitor
            let mut builder = workspace.workspace_monitor()?;
            if let Some(root) = project_root {
                builder = builder.with_project_root(root.to_path_buf());
            }
            let monitor = builder.start_background_scan()?;

            // Create channel for discovery events
            let (tx_discovery, rx_discovery) = channel::<WorkspaceEvent>();

            // Monitor thread: forward discovery events (consume monitor ownership)
            thread::spawn(move || {
                while let Ok(event) = monitor.receiver().recv() {
                    let _ = tx_discovery.send(event);
                }
            });

            // Open new DB connection for session streaming
            let db = agtrace_index::Database::open(workspace.database_path())?;
            let db_mutex = Arc::new(Mutex::new(db));
            let provider_name = name.clone();

            // Stream thread: wait for session discovery and stream events
            thread::spawn(move || {
                let _ = tui_view.render_watch_start(&start_event);
                let _ = tui_view.on_watch_waiting("Waiting for new session...");

                // TODO: Implement SessionRotated functionality (automatic session switching)
                // TODO: Implement Reactor system (TokenUsageMonitor, etc.)

                while let Ok(event) = rx_discovery.recv() {
                    if let WorkspaceEvent::Discovery(DiscoveryEvent::NewSession { summary }) = event
                    {
                        let _ = tui_view.on_watch_attached(&format!("Session {}", summary.id));

                        if let Ok(provider) = agtrace_providers::create_provider(&provider_name) {
                            if let Ok(stream) = SessionStreamer::attach(
                                summary.id.clone(),
                                db_mutex.clone(),
                                Arc::from(provider),
                            ) {
                                let mut session_state: Option<SessionState> = None;
                                let mut initialized = false;

                                let stream_receiver = stream.receiver();
                                while let Ok(stream_event) = stream_receiver.recv() {
                                    match stream_event {
                                        WorkspaceEvent::Stream(StreamEvent::Events { events }) => {
                                            // Initialize state on first events
                                            if session_state.is_none() && !events.is_empty() {
                                                session_state = Some(SessionState::new(
                                                    summary.id.clone(),
                                                    None,
                                                    events[0].timestamp,
                                                ));
                                            }

                                            if let Some(state) = &mut session_state {
                                                // Update state from events
                                                for event in &events {
                                                    state.last_activity = event.timestamp;
                                                    state.event_count += 1;

                                                    let updates =
                                                        agtrace_engine::extract_state_updates(
                                                            event,
                                                        );
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
                                                    let _ = tui_view.on_watch_initial_summary(
                                                        &WatchSummary {
                                                            recent_lines: Vec::new(),
                                                            token_usage: None,
                                                            turn_count: state.turn_count,
                                                        },
                                                    );
                                                    initialized = true;
                                                }

                                                let event_vms = presenters::present_events(&events);
                                                let state_vm =
                                                    presenters::present_session_state(state);
                                                let _ = tui_view
                                                    .render_stream_update(&state_vm, &event_vms);

                                                // TODO: Implement token usage warnings
                                                // (requires Reactor system integration)
                                            }
                                        }
                                        WorkspaceEvent::Stream(StreamEvent::Disconnected {
                                            reason,
                                        }) => {
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
                        }
                        break; // TODO: Remove this to support SessionRotated
                    }
                }
            });
        }
        WatchTarget::Session { id } => {
            let log_root = std::path::PathBuf::new();
            let start_event = WatchStart::Session {
                id: id.clone(),
                log_root,
            };

            // Open new DB connection for thread safety
            let db = agtrace_index::Database::open(workspace.database_path())?;
            let db_mutex = Arc::new(Mutex::new(db));

            // Attach to specific session
            if let Ok(provider) = agtrace_providers::create_provider("claude") {
                let stream = SessionStreamer::attach(id.clone(), db_mutex, Arc::from(provider))?;

                thread::spawn(move || {
                    let _ = tui_view.render_watch_start(&start_event);

                    let mut session_state: Option<SessionState> = None;
                    let mut initialized = false;

                    let stream_receiver = stream.receiver();
                    while let Ok(event) = stream_receiver.recv() {
                        match event {
                            WorkspaceEvent::Stream(StreamEvent::Attached {
                                session_id,
                                path: _,
                            }) => {
                                let _ = tui_view.on_watch_attached(&session_id);
                            }
                            WorkspaceEvent::Stream(StreamEvent::Events { events }) => {
                                // Initialize state on first events
                                if session_state.is_none() && !events.is_empty() {
                                    session_state = Some(SessionState::new(
                                        id.clone(),
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
                                        if let Some(assembled) = assemble_session(&events) {
                                            let _ =
                                                tui_view.on_watch_initial_summary(&WatchSummary {
                                                    recent_lines: Vec::new(),
                                                    token_usage: None,
                                                    turn_count: assembled.turns.len(),
                                                });
                                        }
                                        initialized = true;
                                    }

                                    let event_vms = presenters::present_events(&events);
                                    let state_vm = presenters::present_session_state(state);
                                    let _ = tui_view.render_stream_update(&state_vm, &event_vms);
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
                });
            }
        }
    }

    // Run the TUI event loop on the main thread
    TuiWatchView::run(rx)
}
