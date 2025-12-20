use crate::presentation::view_models::{
    ContextWindowUsageViewModel, ReactionViewModel, StreamStateViewModel,
};

pub fn present_session_state(state: &agtrace_runtime::SessionState) -> StreamStateViewModel {
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

pub fn present_reaction(reaction: &agtrace_runtime::Reaction) -> ReactionViewModel {
    match reaction {
        agtrace_runtime::Reaction::Continue => ReactionViewModel::Continue,
        agtrace_runtime::Reaction::Warn(msg) => ReactionViewModel::Warn(msg.clone()),
    }
}
