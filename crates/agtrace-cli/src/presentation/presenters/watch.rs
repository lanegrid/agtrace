use std::path::Path;

use crate::presentation::view_models::{
    WatchEventViewModel, WatchStreamStateViewModel, WatchTargetViewModel,
};

/// Present watch start event
pub fn present_watch_start_provider(
    provider_name: String,
    log_root: impl AsRef<Path>,
) -> WatchEventViewModel {
    WatchEventViewModel::Start {
        target: WatchTargetViewModel::Provider {
            name: provider_name,
            log_root: log_root.as_ref().to_path_buf(),
        },
    }
}

/// Present watch start event for session
pub fn present_watch_start_session(
    session_id: String,
    log_root: impl AsRef<Path>,
) -> WatchEventViewModel {
    WatchEventViewModel::Start {
        target: WatchTargetViewModel::Session {
            id: session_id,
            log_root: log_root.as_ref().to_path_buf(),
        },
    }
}

/// Present session attached event
pub fn present_watch_attached(session_id: String) -> WatchEventViewModel {
    WatchEventViewModel::Attached { session_id }
}

/// Present session rotated event
pub fn present_watch_rotated(old_session: String, new_session: String) -> WatchEventViewModel {
    WatchEventViewModel::Rotated {
        old_session,
        new_session,
    }
}

/// Present waiting event
pub fn present_watch_waiting(message: String) -> WatchEventViewModel {
    WatchEventViewModel::Waiting { message }
}

/// Present error event
pub fn present_watch_error(message: String, fatal: bool) -> WatchEventViewModel {
    WatchEventViewModel::Error { message, fatal }
}

/// Present stream update event
///
/// Converts domain data (SessionState + Events) into WatchEventViewModel
pub fn present_watch_stream_update(
    state: &agtrace_runtime::SessionState,
    events: &[agtrace_types::AgentEvent],
    assembled_session: Option<&agtrace_engine::AgentSession>,
    max_context: Option<u32>,
) -> WatchEventViewModel {
    use super::lab::present_events;

    // Convert events to view models
    let event_vms = present_events(events);

    // Convert session state to view model
    let state_vm = WatchStreamStateViewModel {
        session_id: state.session_id.clone(),
        project_root: state.project_root.as_ref().map(|p| p.display().to_string()),
        log_path: state.log_path.as_ref().map(|p| p.display().to_string()),
        start_time: state.start_time,
        last_activity: state.last_activity,
        model: state.model.clone(),
        event_count: state.event_count,
        turn_count: state.turn_count,
        current_usage: crate::presentation::view_models::ContextWindowUsageViewModel {
            fresh_input: state.current_usage.fresh_input.0,
            cache_creation: state.current_usage.cache_creation.0,
            cache_read: state.current_usage.cache_read.0,
            output: state.current_usage.output.0,
        },
        token_limit: state.context_window_limit,
        compaction_buffer_pct: None, // Not available in SessionState
    };

    // Build turns data from assembled session (if available)
    let turns = assembled_session.map(|s| build_turns_from_session(s, max_context));

    WatchEventViewModel::StreamUpdate {
        state: state_vm,
        events: event_vms,
        turns,
    }
}

/// Build turn usage view models from assembled session
fn build_turns_from_session(
    session: &agtrace_engine::AgentSession,
    max_context: Option<u32>,
) -> Vec<crate::presentation::view_models::TurnUsageViewModel> {
    use crate::presentation::view_models::{StepItemViewModel, TurnUsageViewModel};

    let metrics = session.compute_turn_metrics(max_context);

    session
        .turns
        .iter()
        .zip(metrics.iter())
        .map(|(turn, metric)| {
            let user_message = &turn.user.content.text;
            let title = truncate_text(user_message, 60);

            let recent_steps = if metric.is_active {
                turn.steps
                    .iter()
                    .rev()
                    .take(5)
                    .rev()
                    .map(|step| {
                        let (emoji, description) = if let Some(reasoning) = &step.reasoning {
                            ("🤔".to_string(), truncate_text(&reasoning.content.text, 40))
                        } else if let Some(message) = &step.message {
                            ("💬".to_string(), truncate_text(&message.content.text, 40))
                        } else if !step.tools.is_empty() {
                            let tool_name = step.tools[0].call.content.name();
                            ("🔧".to_string(), format!("Tool: {}", tool_name))
                        } else {
                            ("•".to_string(), "Event".to_string())
                        };

                        let token_usage = step.usage.as_ref().map(|u| {
                            (u.input_tokens
                                + u.details
                                    .as_ref()
                                    .and_then(|d| d.cache_creation_input_tokens)
                                    .unwrap_or(0)
                                + u.details
                                    .as_ref()
                                    .and_then(|d| d.cache_read_input_tokens)
                                    .unwrap_or(0)) as u32
                        });

                        StepItemViewModel {
                            timestamp: step.timestamp,
                            emoji,
                            description,
                            token_usage,
                        }
                    })
                    .collect()
            } else {
                Vec::new()
            };

            let start_time = turn.steps.first().map(|s| s.timestamp);

            TurnUsageViewModel {
                turn_id: metric.turn_index + 1,
                title,
                prev_total: metric.prev_total,
                delta: metric.delta,
                is_heavy: metric.is_heavy,
                is_active: metric.is_active,
                recent_steps,
                start_time,
            }
        })
        .collect()
}

fn truncate_text(text: &str, max_len: usize) -> String {
    if text.chars().count() <= max_len {
        text.to_string()
    } else {
        let truncated: String = text.chars().take(max_len.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
}
