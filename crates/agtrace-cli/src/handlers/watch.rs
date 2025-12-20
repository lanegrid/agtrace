use crate::context::ExecutionContext;
use crate::presentation::presenters;
use crate::presentation::renderers::traits::WatchView;
use crate::presentation::renderers::{ConsoleTraceView, TuiWatchView};
use crate::presentation::view_models::{
    ContextWindowUsageViewModel, ReactionViewModel, StreamStateViewModel, WatchStart, WatchSummary,
};
use agtrace_runtime::{Runtime, RuntimeConfig, RuntimeEvent, TokenUsageMonitor};
use anyhow::Result;
use is_terminal::IsTerminal;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

pub enum WatchTarget {
    Provider { name: String },
    Session { id: String },
}

fn convert_session_state_to_vm(
    state: &agtrace_runtime::reactor::SessionState,
) -> StreamStateViewModel {
    let token_limits = agtrace_runtime::TokenLimits::new();
    let token_spec = state.model.as_ref().and_then(|m| token_limits.get_limit(m));
    let token_limit = state
        .context_window_limit
        .or_else(|| token_spec.as_ref().map(|spec| spec.effective_limit()));
    let compaction_buffer_pct = token_spec.map(|spec| spec.compaction_buffer_pct);

    StreamStateViewModel {
        session_id: state.session_id.clone(),
        project_root: state.project_root.as_ref().map(|p| p.display().to_string()),
        start_time: state.start_time,
        last_activity: state.last_activity,
        model: state.model.clone(),
        context_window_limit: state.context_window_limit,
        current_usage: ContextWindowUsageViewModel {
            fresh_input: state.current_usage.fresh_input.0,
            cache_creation: state.current_usage.cache_creation.0,
            cache_read: state.current_usage.cache_read.0,
            output: state.current_usage.output.0,
        },
        current_reasoning_tokens: state.current_reasoning_tokens,
        error_count: state.error_count,
        event_count: state.event_count,
        turn_count: state.turn_count,
        token_limit,
        compaction_buffer_pct,
    }
}

fn convert_reaction_to_vm(reaction: &agtrace_runtime::reactor::Reaction) -> ReactionViewModel {
    match reaction {
        agtrace_runtime::reactor::Reaction::Continue => ReactionViewModel::Continue,
        agtrace_runtime::reactor::Reaction::Warn(msg) => ReactionViewModel::Warn(msg.clone()),
    }
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
    let (provider, log_root, explicit_target, start_event): (
        Arc<dyn agtrace_providers::LogProvider>,
        PathBuf,
        Option<String>,
        WatchStart,
    ) = match target {
        WatchTarget::Provider { name } => {
            let (provider, log_root) = ctx.resolve_provider(&name)?;
            (
                Arc::from(provider),
                log_root.clone(),
                None,
                WatchStart::Provider {
                    name,
                    log_root: log_root.clone(),
                },
            )
        }
        WatchTarget::Session { id } => {
            let provider_name = ctx.default_provider()?;
            let (provider, log_root) = ctx.resolve_provider(&provider_name)?;
            (
                Arc::from(provider),
                log_root.clone(),
                Some(id.clone()),
                WatchStart::Session { id, log_root },
            )
        }
    };

    view.render_watch_start(&start_event)?;

    let project_root = if explicit_target.is_some() {
        None
    } else {
        ctx.project_root.clone()
    };

    let runtime = Runtime::start(RuntimeConfig {
        provider,
        reactors: vec![Box::new(TokenUsageMonitor::default_thresholds())],
        watch_path: log_root,
        explicit_target,
        project_root,
        poll_interval: Duration::from_millis(500),
    })?;

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
                let state_vm = convert_session_state_to_vm(&state);
                view.render_stream_update(&state_vm, &event_vms)?;
            }
            RuntimeEvent::ReactionTriggered { reaction, .. } => {
                let reaction_vm = convert_reaction_to_vm(&reaction);
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
