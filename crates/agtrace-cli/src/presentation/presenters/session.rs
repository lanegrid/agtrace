use crate::presentation::view_models::{
    SessionDigestViewModel, SessionStatsViewModel, SessionViewModel, StepViewModel,
    TokenUsageViewModel, ToolExecutionViewModel, TurnStatsViewModel, TurnViewModel,
};
use agtrace_engine::{AgentSession, AgentStep, AgentTurn, SessionDigest};

pub fn present_session(session: &AgentSession) -> SessionViewModel {
    SessionViewModel {
        session_id: session.session_id.to_string(),
        start_time: session.start_time,
        end_time: session.end_time,
        turns: session.turns.iter().map(present_turn).collect(),
        stats: SessionStatsViewModel {
            total_turns: session.turns.len(),
            total_steps: session.turns.iter().map(|t| t.steps.len()).sum(),
            total_tool_calls: session
                .turns
                .iter()
                .flat_map(|t| &t.steps)
                .flat_map(|s| &s.tools)
                .count(),
        },
    }
}

pub fn present_turn(turn: &AgentTurn) -> TurnViewModel {
    TurnViewModel {
        id: turn.id.to_string(),
        timestamp: turn.timestamp,
        user_message: turn.user.content.text.clone(),
        steps: turn.steps.iter().map(present_step).collect(),
        stats: TurnStatsViewModel {
            total_steps: turn.steps.len(),
            total_tool_calls: turn.steps.iter().flat_map(|s| &s.tools).count(),
        },
    }
}

pub fn present_step(step: &AgentStep) -> StepViewModel {
    StepViewModel {
        id: step.id.to_string(),
        timestamp: step.timestamp,
        reasoning_text: step.reasoning.as_ref().map(|r| r.content.text.clone()),
        message_text: step.message.as_ref().map(|m| m.content.text.clone()),
        tools: step
            .tools
            .iter()
            .map(|t| ToolExecutionViewModel {
                name: t.call.content.name.clone(),
                arguments: t.call.content.arguments.clone(),
                output: t.result.as_ref().map(|r| r.content.output.clone()),
                duration_ms: t.duration_ms,
                is_error: t.is_error,
            })
            .collect(),
        usage: step.usage.as_ref().map(|u| TokenUsageViewModel {
            input_tokens: u.input_tokens,
            output_tokens: u.output_tokens,
            total_tokens: u.total_tokens,
            cache_creation_tokens: u
                .details
                .as_ref()
                .and_then(|d| d.cache_creation_input_tokens),
            cache_read_tokens: u.details.as_ref().and_then(|d| d.cache_read_input_tokens),
        }),
        is_failed: step.is_failed,
    }
}

pub fn present_digest(digest: &SessionDigest) -> SessionDigestViewModel {
    SessionDigestViewModel {
        session_id: digest.session_id.clone(),
        source: digest.source.clone(),
        opening: digest.opening.clone(),
        activation: digest.activation.clone(),
        tool_calls_total: digest.metrics.tool_calls_total,
        tool_failures_total: digest.metrics.tool_failures_total,
        max_e2e_ms: digest.metrics.max_e2e_ms,
        max_tool_ms: digest.metrics.max_tool_ms,
        missing_tool_pairs: digest.metrics.missing_tool_pairs,
        loop_signals: digest.metrics.loop_signals,
        longest_chain: digest.metrics.longest_chain,
        recency_boost: digest.recency_boost,
        selection_reason: digest.selection_reason.clone(),
    }
}
