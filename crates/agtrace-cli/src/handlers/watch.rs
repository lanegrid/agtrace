use crate::context::ExecutionContext;
use crate::presentation::presenters;
use crate::presentation::renderers::traits::WatchView;
use crate::presentation::renderers::{ConsoleTraceView, TuiWatchView};
use crate::presentation::view_models::{WatchStart, WatchSummary};
use agtrace_runtime::{RuntimeEvent, WatchConfig, WatchService};
use anyhow::Result;
use is_terminal::IsTerminal;
use std::sync::Arc;

pub enum WatchTarget {
    Provider { name: String },
    Session { id: String },
}

pub fn handle(ctx: &ExecutionContext, target: WatchTarget) -> Result<()> {
    // Auto-select TUI mode if stdout is a TTY
    let use_tui = std::io::stdout().is_terminal();

    if use_tui {
        let tui_view = TuiWatchView::new()?;
        handle_with_view(ctx, target, &tui_view)
    } else {
        let console_view = ConsoleTraceView::new();
        handle_with_view(ctx, target, &console_view)
    }
}

pub fn handle_with_view(
    ctx: &ExecutionContext,
    target: WatchTarget,
    view: &dyn WatchView,
) -> Result<()> {
    let (config, start_event) = match target {
        WatchTarget::Provider { name } => {
            let (provider, log_root) = ctx.resolve_provider(&name)?;
            let config = WatchConfig {
                provider: Arc::from(provider),
                log_root: log_root.clone(),
                explicit_target: None,
                project_root: ctx.project_root.clone(),
                enable_token_monitor: true,
            };
            let start = WatchStart::Provider {
                name,
                log_root: log_root.clone(),
            };
            (config, start)
        }
        WatchTarget::Session { id } => {
            let provider_name = ctx.default_provider()?;
            let (provider, log_root) = ctx.resolve_provider(&provider_name)?;
            let config = WatchConfig {
                provider: Arc::from(provider),
                log_root: log_root.clone(),
                explicit_target: Some(id.clone()),
                project_root: None,
                enable_token_monitor: true,
            };
            let start = WatchStart::Session { id, log_root };
            (config, start)
        }
    };

    view.render_watch_start(&start_event)?;

    let runtime = WatchService::start(config)?;
    let mut initialized = false;

    for event in runtime.receiver().iter() {
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
