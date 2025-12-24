use crate::presentation::view_models::{
    AgentStepViewModel, CommandResultViewModel, ContextUsage, ContextWindowSummary,
    ContextWindowUsageViewModel, FilterSummary, Guidance, SessionAnalysisViewModel, SessionHeader,
    SessionListEntry, SessionListViewModel, StatusBadge, StreamStateViewModel,
    TurnAnalysisViewModel, TurnMetrics as ViewTurnMetrics,
};
use agtrace_engine::AgentSession;
use agtrace_index::SessionSummary;

pub fn present_session_list(
    sessions: Vec<SessionSummary>,
    project_filter: Option<String>,
    source_filter: Option<String>,
    time_range: Option<String>,
    limit: usize,
) -> CommandResultViewModel<SessionListViewModel> {
    let view = build_session_list_view(sessions, project_filter, source_filter, time_range, limit);
    let result = CommandResultViewModel::new(view);
    add_session_list_guidance(result)
}

fn build_session_list_view(
    sessions: Vec<SessionSummary>,
    project_filter: Option<String>,
    source_filter: Option<String>,
    time_range: Option<String>,
    limit: usize,
) -> SessionListViewModel {
    let total_count = sessions.len();

    let entries: Vec<SessionListEntry> = sessions
        .into_iter()
        .map(|s| SessionListEntry {
            id: s.id,
            provider: s.provider,
            project_hash: s.project_hash,
            start_ts: s.start_ts,
            snippet: s.snippet,
        })
        .collect();

    SessionListViewModel {
        sessions: entries,
        total_count,
        applied_filters: FilterSummary {
            project_filter,
            source_filter,
            time_range,
            limit,
        },
    }
}

fn add_session_list_guidance(
    mut result: CommandResultViewModel<SessionListViewModel>,
) -> CommandResultViewModel<SessionListViewModel> {
    let total_count = result.content.total_count;
    let limit = result.content.applied_filters.limit;

    if total_count == 0 {
        result = result
            .with_badge(StatusBadge::info("No sessions found"))
            .with_suggestion(
                Guidance::new("Index sessions to populate the database")
                    .with_command("agtrace index update"),
            )
            .with_suggestion(
                Guidance::new("Or scan all projects")
                    .with_command("agtrace index update --all-projects"),
            );
    } else {
        let label = if total_count == 1 {
            "1 session found".to_string()
        } else {
            format!("{} sessions found", total_count)
        };
        result = result.with_badge(StatusBadge::success(label));

        if total_count >= limit {
            result = result.with_suggestion(
                Guidance::new(format!(
                    "Showing first {} sessions, use --limit to see more",
                    limit
                ))
                .with_command(format!("agtrace session list --limit {}", limit * 2)),
            );
        }
    }

    result
}

/// Present session analysis with context-aware metrics
pub fn present_session_analysis(
    session: &AgentSession,
    provider: &str,
    model: &str,
    max_context: Option<u32>,
) -> CommandResultViewModel<SessionAnalysisViewModel> {
    let view = build_session_analysis_view(session, provider, model, max_context);
    let result = CommandResultViewModel::new(view);
    add_session_analysis_guidance(result)
}

fn build_session_analysis_view(
    session: &AgentSession,
    provider: &str,
    model: &str,
    max_context: Option<u32>,
) -> SessionAnalysisViewModel {
    use crate::presentation::formatters::time;

    let metrics = session.compute_turn_metrics(max_context);

    // Compute duration
    let duration = if let (Some(first_turn), Some(last_turn)) =
        (session.turns.first(), session.turns.last())
    {
        if let (Some(first_step), Some(last_step)) =
            (first_turn.steps.first(), last_turn.steps.last())
        {
            Some(time::format_duration(
                first_step.timestamp,
                last_step.timestamp,
            ))
        } else {
            None
        }
    } else {
        None
    };

    // Compute start time
    let start_time = session
        .turns
        .first()
        .and_then(|t| t.steps.first().map(|s| time::format_time(s.timestamp)));

    // Build header
    let header = SessionHeader {
        session_id: session.session_id.to_string(),
        provider: provider.to_string(),
        model: Some(model.to_string()),
        status: if session.turns.is_empty() {
            "Empty".to_string()
        } else {
            "Complete".to_string()
        },
        duration,
        start_time,
    };

    // Build context summary (raw data only)
    let total_tokens = metrics.last().map(|m| m.prev_total + m.delta).unwrap_or(0);
    let context_summary = ContextWindowSummary {
        current_tokens: total_tokens,
        max_tokens: max_context,
    };

    // Build turns
    let turns = session
        .turns
        .iter()
        .zip(metrics.iter())
        .map(|(turn, metric)| build_turn_analysis(turn, metric, max_context))
        .collect();

    SessionAnalysisViewModel {
        header,
        context_summary,
        turns,
    }
}

fn add_session_analysis_guidance(
    result: CommandResultViewModel<SessionAnalysisViewModel>,
) -> CommandResultViewModel<SessionAnalysisViewModel> {
    result.with_badge(StatusBadge::success("Session Analysis"))
}

fn build_turn_analysis(
    turn: &agtrace_engine::AgentTurn,
    metric: &agtrace_engine::TurnMetrics,
    max_context: Option<u32>,
) -> TurnAnalysisViewModel {
    use crate::presentation::formatters::time;

    let user_query = turn.user.content.text.clone();
    let prev_tokens = metric.prev_total;
    let current_tokens = metric.prev_total + metric.delta;

    // Build context usage data (only if max_context is known)
    let context_usage = max_context.map(|max| {
        let current_tokens = metric.prev_total + metric.delta;
        let percentage = (current_tokens as f64 / max as f64) * 100.0;
        ContextUsage {
            current_tokens,
            max_tokens: max,
            percentage,
        }
    });

    // Build steps from turn
    let steps = turn
        .steps
        .iter()
        .flat_map(|step| {
            let mut result = Vec::new();

            if let Some(ref reasoning) = step.reasoning {
                result.push(AgentStepViewModel::Thinking {
                    duration: None,
                    preview: reasoning.content.text.clone(),
                });
            }

            if !step.tools.is_empty() {
                // Group consecutive tool calls with the same name
                let grouped_tools = group_consecutive_tools(&step.tools);
                for group in grouped_tools {
                    result.extend(group);
                }
            }

            if let Some(ref message) = step.message {
                result.push(AgentStepViewModel::Message {
                    text: message.content.text.clone(),
                });
            }

            result
        })
        .collect();

    // Calculate metrics
    let total_input: i64 = turn
        .steps
        .iter()
        .filter_map(|s| s.usage.as_ref())
        .map(|u| {
            u.input_tokens as i64
                + u.details
                    .as_ref()
                    .and_then(|d| d.cache_creation_input_tokens)
                    .unwrap_or(0) as i64
        })
        .sum();

    let total_output: i64 = turn
        .steps
        .iter()
        .filter_map(|s| s.usage.as_ref())
        .map(|u| u.output_tokens as i64)
        .sum();

    let cache_read_total: i64 = turn
        .steps
        .iter()
        .filter_map(|s| s.usage.as_ref())
        .filter_map(|u| u.details.as_ref())
        .filter_map(|d| d.cache_read_input_tokens)
        .map(|v| v as i64)
        .sum();

    TurnAnalysisViewModel {
        turn_number: metric.turn_index + 1,
        timestamp: turn.steps.first().map(|s| time::format_time(s.timestamp)),
        prev_tokens,
        current_tokens,
        context_usage,
        is_heavy_load: metric.is_heavy,
        user_query,
        steps,
        metrics: ViewTurnMetrics {
            total_delta: metric.delta,
            input_tokens: total_input,
            output_tokens: total_output,
            cache_read_tokens: if cache_read_total > 0 {
                Some(cache_read_total)
            } else {
                None
            },
        },
    }
}

fn group_consecutive_tools(
    tools: &[agtrace_engine::ToolExecution],
) -> Vec<Vec<AgentStepViewModel>> {
    let mut result: Vec<Vec<AgentStepViewModel>> = Vec::new();
    let mut current_group: Vec<&agtrace_engine::ToolExecution> = Vec::new();
    let mut current_name: Option<String> = None;

    for tool in tools {
        let name = tool.call.content.name().to_string();

        if current_name.as_ref() == Some(&name) {
            // Same tool, add to current group
            current_group.push(tool);
        } else {
            // Different tool, flush current group
            if !current_group.is_empty() {
                result.push(create_tool_view_models(&current_group));
            }
            current_group = vec![tool];
            current_name = Some(name);
        }
    }

    // Flush final group
    if !current_group.is_empty() {
        result.push(create_tool_view_models(&current_group));
    }

    result
}

fn create_tool_view_models(tools: &[&agtrace_engine::ToolExecution]) -> Vec<AgentStepViewModel> {
    if tools.is_empty() {
        return vec![];
    }

    // If 3+ consecutive calls with same name, create a ToolCallSequence
    if tools.len() >= 3 {
        let name = tools[0].call.content.name().to_string();
        let sample_arguments = tools[0].call.content.clone();
        let has_errors = tools.iter().any(|t| {
            t.result
                .as_ref()
                .map(|r| r.content.is_error)
                .unwrap_or(false)
        });

        vec![AgentStepViewModel::ToolCallSequence {
            name,
            count: tools.len(),
            sample_arguments,
            sample_args_formatted: None,
            has_errors,
        }]
    } else {
        // Otherwise, create individual ToolCall entries
        tools
            .iter()
            .map(|tool| {
                let name = tool.call.content.name().to_string();
                let arguments = tool.call.content.clone();
                let is_error = tool
                    .result
                    .as_ref()
                    .map(|r| r.content.is_error)
                    .unwrap_or(false);
                let result_text = tool
                    .result
                    .as_ref()
                    .map(|r| r.content.output.clone())
                    .unwrap_or_else(|| "(no result)".to_string());

                AgentStepViewModel::ToolCall {
                    name,
                    arguments,
                    args_formatted: None,
                    result: result_text,
                    is_error,
                }
            })
            .collect()
    }
}

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
