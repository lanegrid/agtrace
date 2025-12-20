use crate::context::ExecutionContext;
use crate::presentation::presenters;
use crate::presentation::renderers::traits::WatchView;
use crate::presentation::renderers::{ConsoleTraceView, TuiWatchView};
use crate::presentation::view_models::{WatchStart, WatchSummary};
use agtrace_runtime::RuntimeEvent;
use anyhow::Result;
use is_terminal::IsTerminal;

pub enum WatchTarget {
    Provider { name: String },
    Session { id: String },
}

pub fn handle(ctx: &ExecutionContext, target: WatchTarget) -> Result<()> {
    // Auto-select TUI mode if stdout is a TTY
    let use_tui = std::io::stdout().is_terminal();

    if use_tui {
        handle_tui(ctx, target)
    } else {
        let console_view = ConsoleTraceView::new();
        handle_with_view(ctx, target, &console_view)
    }
}

/// Handle watch command in TUI mode
fn handle_tui(ctx: &ExecutionContext, target: WatchTarget) -> Result<()> {
    use std::thread;

    // Create TUI view and get receiver for event loop
    let (tui_view, rx) = TuiWatchView::new()?;
    let workspace = ctx.workspace()?;

    // Prepare RuntimeBuilder configuration
    let (runtime, start_event) = match target {
        WatchTarget::Provider { name } => {
            let mut builder = workspace
                .monitor()
                .with_provider(&name)
                .with_token_monitor()
                .watch_latest();

            if let Some(root) = ctx.project_root.clone() {
                builder = builder.with_project_root(root);
            }

            let runtime = builder.start()?;

            let log_root = workspace
                .config()
                .providers
                .get(&name)
                .map(|p| p.log_root.clone())
                .unwrap_or_default();

            let start = WatchStart::Provider { name, log_root };
            (runtime, start)
        }
        WatchTarget::Session { id } => {
            let runtime = workspace.monitor().watch_session(&id).start()?;

            let log_root = std::path::PathBuf::new();
            let start = WatchStart::Session { id, log_root };
            (runtime, start)
        }
    };

    // Start runtime in background thread
    thread::spawn(move || {
        // Send initial start event
        let _ = tui_view.render_watch_start(&start_event);

        let mut initialized = false;

        // Process events from RuntimeBuilder
        while let Some(event) = runtime.next_event() {
            match event {
                RuntimeEvent::SessionAttached { display_name } => {
                    let _ = tui_view.on_watch_attached(&display_name);
                }
                RuntimeEvent::StateUpdated { state, new_events } => {
                    if !initialized {
                        let _ = tui_view.on_watch_initial_summary(&WatchSummary {
                            recent_lines: Vec::new(),
                            token_usage: None,
                            turn_count: state.turn_count,
                        });
                        initialized = true;
                    }
                    let event_vms = presenters::present_events(&new_events);
                    let state_vm = presenters::present_session_state(&state);
                    let _ = tui_view.render_stream_update(&state_vm, &event_vms);
                }
                RuntimeEvent::ReactionTriggered { reaction, .. } => {
                    let reaction_vm = presenters::present_reaction(&reaction);
                    let _ = tui_view.on_watch_reaction(&reaction_vm);
                }
                RuntimeEvent::SessionRotated { old_path, new_path } => {
                    initialized = false;
                    let _ = tui_view.on_watch_rotated(&old_path, &new_path);
                }
                RuntimeEvent::Waiting { message } => {
                    let _ = tui_view.on_watch_waiting(&message);
                }
                RuntimeEvent::FatalError(msg) => {
                    let fatal = msg.starts_with("FATAL:");
                    let _ = tui_view.on_watch_error(&msg, fatal);
                    if fatal {
                        break;
                    }
                }
            }
        }
    });

    // Run the TUI event loop on the main thread
    TuiWatchView::run(rx)
}

pub fn handle_with_view(
    ctx: &ExecutionContext,
    target: WatchTarget,
    view: &dyn WatchView,
) -> Result<()> {
    let workspace = ctx.workspace()?;

    let (runtime, start_event) = match target {
        WatchTarget::Provider { name } => {
            let mut builder = workspace
                .monitor()
                .with_provider(&name)
                .with_token_monitor()
                .watch_latest();

            if let Some(root) = ctx.project_root.clone() {
                builder = builder.with_project_root(root);
            }

            let runtime = builder.start()?;

            let log_root = workspace
                .config()
                .providers
                .get(&name)
                .map(|p| p.log_root.clone())
                .unwrap_or_default();

            let start = WatchStart::Provider { name, log_root };
            (runtime, start)
        }
        WatchTarget::Session { id } => {
            let runtime = workspace.monitor().watch_session(&id).start()?;

            let log_root = std::path::PathBuf::new();
            let start = WatchStart::Session { id, log_root };
            (runtime, start)
        }
    };

    view.render_watch_start(&start_event)?;

    let mut initialized = false;

    while let Some(event) = runtime.next_event() {
        match event {
            RuntimeEvent::SessionAttached { display_name } => {
                view.on_watch_attached(&display_name)?
            }
            RuntimeEvent::StateUpdated { state, new_events } => {
                if !initialized {
                    view.on_watch_initial_summary(&WatchSummary {
                        recent_lines: Vec::new(),
                        token_usage: None,
                        turn_count: state.turn_count,
                    })?;
                    initialized = true;
                }
                let event_vms = presenters::present_events(&new_events);
                let state_vm = presenters::present_session_state(&state);
                view.render_stream_update(&state_vm, &event_vms)?;
            }
            RuntimeEvent::ReactionTriggered { reaction, .. } => {
                let reaction_vm = presenters::present_reaction(&reaction);
                view.on_watch_reaction(&reaction_vm)?
            }
            RuntimeEvent::SessionRotated { old_path, new_path } => {
                initialized = false;
                view.on_watch_rotated(&old_path, &new_path)?;
            }
            RuntimeEvent::Waiting { message } => view.on_watch_waiting(&message)?,
            RuntimeEvent::FatalError(msg) => {
                let fatal = msg.starts_with("FATAL:");
                view.on_watch_error(&msg, fatal)?;
                if fatal {
                    break;
                }
            }
        }
    }

    Ok(())
}
