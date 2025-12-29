/// Console watch handler - streams WatchEventViewModel to stdout
use crate::presentation::presenters::watch as present_watch;
use crate::presentation::view_models::{ViewMode, WatchEventViewModel};
use crate::presentation::views::watch::WatchEventView;
use agtrace_runtime::{AgTrace, DiscoveryEvent, SessionState, StreamEvent, WorkspaceEvent};
use anyhow::Result;
use std::collections::VecDeque;
use std::path::Path;
use std::sync::mpsc::RecvTimeoutError;
use std::time::Duration;

use super::watch_tui::WatchTarget;

pub fn handle_console(
    workspace: &AgTrace,
    project_root: Option<&Path>,
    target: WatchTarget,
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

            // Print start event
            let start_event = present_watch::present_watch_start_provider(name.clone(), &log_root);
            print_event(&start_event, ViewMode::Standard);

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
                    provider_filter: Some(name.clone()),
                };

                // Lightweight scan (incremental by default)
                let _ = workspace.projects().scan(&scan_context, false, |_| {});

                // Get latest session from updated DB
                let filter = SessionFilter::new().provider(name.clone()).limit(1);
                workspace
                    .sessions()
                    .list(filter)
                    .ok()
                    .and_then(|sessions| sessions.into_iter().next())
            };

            // NOTE: Watch all providers for cross-provider session switching
            // - Previously: only watched single provider (name)
            // - Now: watch all enabled providers to detect new sessions from any provider
            // - This enables real-time switching to sessions from different providers
            let mut builder = watch_service.watch_all_providers()?;
            if let Some(root) = project_root {
                builder = builder.with_project_root(root.to_path_buf());
            }
            let monitor = builder.start_background_scan()?;

            // Create channel for discovery events
            let (tx_discovery, rx_discovery) = std::sync::mpsc::channel::<WorkspaceEvent>();

            // Monitor thread: forward discovery events
            thread::spawn(move || {
                while let Ok(event) = monitor.receiver().recv() {
                    let _ = tx_discovery.send(event);
                }
            });

            // Clone watch_service for thread (cheap Arc clone)
            let watch_service_clone = watch_service.clone();

            // Process events on main thread (so we can print to stdout)
            process_provider_events_console(
                &watch_service_clone,
                rx_discovery,
                latest_session,
                project_root,
            );

            Ok(())
        }
        WatchTarget::Session { id } => {
            let log_root = std::path::PathBuf::new();
            let start_event = present_watch::present_watch_start_session(id.clone(), &log_root);
            print_event(&start_event, ViewMode::Standard);

            // Attach to session
            let handle = watch_service.watch_session(&id)?;

            process_stream_events_console(handle.receiver(), id, project_root);

            Ok(())
        }
    }
}

fn process_provider_events_console(
    watch_service: &agtrace_runtime::WatchService,
    rx_discovery: std::sync::mpsc::Receiver<WorkspaceEvent>,
    initial_session: Option<agtrace_index::SessionSummary>,
    project_root: Option<&Path>,
) {
    let mut current_handle: Option<agtrace_runtime::StreamHandle> = None;
    let mut current_session_id: Option<String> = None;
    let mut current_session_mod_time: Option<String> = None;
    let mut session_state: Option<SessionState> = None;
    let mut current_log_path: Option<std::path::PathBuf> = None;
    let mut event_buffer: VecDeque<agtrace_types::AgentEvent> = VecDeque::new();
    let mut assembled_session: Option<agtrace_engine::AgentSession> = None;
    let project_root_buf = project_root.map(|p| p.to_path_buf());
    let poll_timeout = Duration::from_millis(100);

    // Attach to initial session if available
    if let Some(session) = initial_session {
        let event = present_watch::present_watch_attached(session.id.clone());
        print_event(&event, ViewMode::Compact);

        match watch_service.watch_session(&session.id) {
            Ok(handle) => {
                current_handle = Some(handle);
                current_session_id = Some(session.id.clone());
            }
            Err(e) => {
                let error_event =
                    present_watch::present_watch_error(format!("Failed to attach: {}", e), false);
                print_event(&error_event, ViewMode::Compact);
            }
        }
    } else {
        let waiting_event =
            present_watch::present_watch_waiting("Waiting for new session...".to_string());
        print_event(&waiting_event, ViewMode::Compact);
    }

    loop {
        // Check for new session discoveries (non-blocking)
        match rx_discovery.try_recv() {
            Ok(WorkspaceEvent::Discovery(DiscoveryEvent::NewSession { summary })) => {
                let event = present_watch::present_watch_attached(summary.id.clone());
                print_event(&event, ViewMode::Compact);

                // Create new session handle
                match watch_service.watch_session(&summary.id) {
                    Ok(handle) => {
                        current_handle = Some(handle);
                        current_session_id = Some(summary.id.clone());
                        session_state = None;
                        event_buffer.clear();
                        assembled_session = None;
                    }
                    Err(e) => {
                        let error_event = present_watch::present_watch_error(
                            format!("Failed to attach: {}", e),
                            false,
                        );
                        print_event(&error_event, ViewMode::Compact);
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
                // Same logic as watch_tui: use mod_time instead of is_new flag
                let should_switch = if let Some(ref new_mod_time) = mod_time {
                    current_session_id.as_ref() != Some(&session_id)
                        && (current_session_mod_time.is_none()
                            || Some(new_mod_time) > current_session_mod_time.as_ref())
                } else {
                    false
                };

                if should_switch {
                    let event = present_watch::present_watch_attached(session_id.clone());
                    print_event(&event, ViewMode::Compact);

                    match watch_service.watch_session(&session_id) {
                        Ok(handle) => {
                            current_handle = Some(handle);
                            current_session_id = Some(session_id.clone());
                            current_session_mod_time = mod_time.clone();
                            session_state = None;
                            event_buffer.clear();
                            assembled_session = None;
                        }
                        Err(e) => {
                            let error_event = present_watch::present_watch_error(
                                format!("Failed to attach: {}", e),
                                false,
                            );
                            print_event(&error_event, ViewMode::Compact);
                        }
                    }
                }
            }
            Ok(WorkspaceEvent::Error(msg)) => {
                let fatal = msg.starts_with("FATAL:");
                let error_event = present_watch::present_watch_error(msg, fatal);
                print_event(&error_event, ViewMode::Compact);
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
                Ok(WorkspaceEvent::Stream(StreamEvent::Attached { session_id, path })) => {
                    current_log_path = Some(path);
                    let event = present_watch::present_watch_attached(session_id);
                    print_event(&event, ViewMode::Compact);
                }
                Ok(WorkspaceEvent::Stream(StreamEvent::Events { events, session })) => {
                    const MAX_EVENTS: usize = 100;

                    if session_state.is_none() && !events.is_empty() {
                        let session_id = current_session_id
                            .clone()
                            .unwrap_or_else(|| "unknown".to_string());

                        session_state = Some(SessionState::new(
                            session_id,
                            project_root_buf.clone(),
                            current_log_path.clone(),
                            events[0].timestamp,
                        ));
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
                            if let Some(model) = updates.model
                                && state.model.is_none()
                            {
                                state.model = Some(model);
                            }
                            if let Some(limit) = updates.context_window_limit {
                                state.context_window_limit = Some(limit);
                            }

                            // Add to event buffer
                            event_buffer.push_back(event.clone());
                            if event_buffer.len() > MAX_EVENTS {
                                event_buffer.pop_front();
                            }
                        }

                        // Update assembled session
                        if let Some(sess) = session {
                            assembled_session = Some(sess);
                        }

                        // Build max_context from state
                        let token_limits = agtrace_runtime::TokenLimits::new();
                        let token_spec =
                            state.model.as_ref().and_then(|m| token_limits.get_limit(m));
                        let max_context = state
                            .context_window_limit
                            .or_else(|| token_spec.as_ref().map(|spec| spec.effective_limit()))
                            .map(|c| c as u32);

                        let update_event = present_watch::present_watch_stream_update(
                            state,
                            &event_buffer,
                            assembled_session.as_ref(),
                            max_context,
                            None, // no notification for console mode
                        );
                        print_event(&update_event, ViewMode::Standard);
                    }
                }
                Ok(WorkspaceEvent::Stream(StreamEvent::Disconnected { reason })) => {
                    let error_event = present_watch::present_watch_error(reason, false);
                    print_event(&error_event, ViewMode::Compact);
                    current_handle = None;

                    let waiting_event = present_watch::present_watch_waiting(
                        "Waiting for new session...".to_string(),
                    );
                    print_event(&waiting_event, ViewMode::Compact);
                }
                Ok(WorkspaceEvent::Error(msg)) => {
                    let fatal = msg.starts_with("FATAL:");
                    let error_event = present_watch::present_watch_error(msg, fatal);
                    print_event(&error_event, ViewMode::Compact);
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

fn process_stream_events_console(
    receiver: &std::sync::mpsc::Receiver<WorkspaceEvent>,
    session_id: String,
    project_root: Option<&Path>,
) {
    let mut session_state: Option<SessionState> = None;
    let mut current_log_path: Option<std::path::PathBuf> = None;
    let mut event_buffer: VecDeque<agtrace_types::AgentEvent> = VecDeque::new();
    let mut assembled_session: Option<agtrace_engine::AgentSession> = None;
    let project_root_buf = project_root.map(|p| p.to_path_buf());

    while let Ok(event) = receiver.recv() {
        match event {
            WorkspaceEvent::Stream(StreamEvent::Attached { session_id, path }) => {
                current_log_path = Some(path);
                let event = present_watch::present_watch_attached(session_id);
                print_event(&event, ViewMode::Compact);
            }
            WorkspaceEvent::Stream(StreamEvent::Events { events, session }) => {
                const MAX_EVENTS: usize = 100;

                // Initialize state on first events
                if session_state.is_none() && !events.is_empty() {
                    session_state = Some(SessionState::new(
                        session_id.clone(),
                        project_root_buf.clone(),
                        current_log_path.clone(),
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
                        if let Some(model) = updates.model
                            && state.model.is_none()
                        {
                            state.model = Some(model);
                        }
                        if let Some(limit) = updates.context_window_limit {
                            state.context_window_limit = Some(limit);
                        }

                        // Add to event buffer
                        event_buffer.push_back(event.clone());
                        if event_buffer.len() > MAX_EVENTS {
                            event_buffer.pop_front();
                        }
                    }

                    // Update assembled session
                    if let Some(sess) = session {
                        assembled_session = Some(sess);
                    }

                    // Build max_context from state
                    let token_limits = agtrace_runtime::TokenLimits::new();
                    let token_spec = state.model.as_ref().and_then(|m| token_limits.get_limit(m));
                    let max_context = state
                        .context_window_limit
                        .or_else(|| token_spec.as_ref().map(|spec| spec.effective_limit()))
                        .map(|c| c as u32);

                    let update_event = present_watch::present_watch_stream_update(
                        state,
                        &event_buffer,
                        assembled_session.as_ref(),
                        max_context,
                        None, // no notification for console mode
                    );
                    print_event(&update_event, ViewMode::Standard);
                }
            }
            WorkspaceEvent::Stream(StreamEvent::Disconnected { reason }) => {
                let error_event = present_watch::present_watch_error(reason, false);
                print_event(&error_event, ViewMode::Compact);
                break;
            }
            WorkspaceEvent::Error(msg) => {
                let fatal = msg.starts_with("FATAL:");
                let error_event = present_watch::present_watch_error(msg, fatal);
                print_event(&error_event, ViewMode::Compact);
                if fatal {
                    break;
                }
            }
            _ => {}
        }
    }
}

fn print_event(event: &WatchEventViewModel, mode: ViewMode) {
    let view = WatchEventView::new(event, mode);
    print!("{}", view);
    // Flush to ensure immediate output
    use std::io::Write;
    let _ = std::io::stdout().flush();
}
