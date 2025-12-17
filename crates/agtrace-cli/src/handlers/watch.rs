use crate::context::ExecutionContext;
use crate::intervention::CliInterventionExecutor;
use crate::reactors::{SafetyGuard, StallDetector, TokenUsageMonitor};
use crate::ui::models::{WatchStart, WatchSummary};
use crate::ui::TraceView;
use agtrace_runtime::{Runtime, RuntimeConfig, RuntimeEvent};
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

pub enum WatchTarget {
    Provider { name: String },
    Session { id: String },
}
pub fn handle(ctx: &ExecutionContext, target: WatchTarget, view: &dyn TraceView) -> Result<()> {
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
        reactors: vec![
            Box::new(StallDetector::new(60)),
            Box::new(SafetyGuard::new()),
            Box::new(TokenUsageMonitor::default_thresholds()),
        ],
        executor: Arc::new(CliInterventionExecutor::new()),
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
                view.render_stream_update(&state, &new_events)?;
            }
            RuntimeEvent::ReactionTriggered { reaction, .. } => {
                view.on_watch_reaction(&reaction)?
            }
            RuntimeEvent::InterventionExecuted {
                intervention,
                result,
            } => {
                let msg = match result {
                    Ok(()) => format!("Intervention executed: {:?}", intervention),
                    Err(e) => format!("Intervention failed: {:?} ({})", intervention, e),
                };
                view.render_warning(&msg)?;
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
