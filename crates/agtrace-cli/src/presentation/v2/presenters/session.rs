use crate::presentation::v2::view_models::{
    AgentStepViewModel, CommandResultViewModel, ContextUsage, ContextWindowSummary, FilterSummary,
    Guidance, SessionAnalysisViewModel, SessionHeader, SessionListEntry, SessionListViewModel,
    StatusBadge, TurnAnalysisViewModel, TurnMetrics as ViewTurnMetrics,
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

    let content = SessionListViewModel {
        sessions: entries,
        total_count,
        applied_filters: FilterSummary {
            project_filter,
            source_filter,
            time_range,
            limit,
        },
    };

    let mut result = CommandResultViewModel::new(content);

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
    let metrics = session.compute_turn_metrics(max_context);

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
        duration: compute_duration(session),
        start_time: session
            .turns
            .first()
            .and_then(|t| t.steps.first().map(|s| format_timestamp(s.timestamp))),
    };

    // Build context summary
    let total_tokens = metrics.last().map(|m| m.prev_total + m.delta).unwrap_or(0);
    let context_summary = build_context_summary(total_tokens, max_context);

    // Build turns
    let turns = session
        .turns
        .iter()
        .zip(metrics.iter())
        .map(|(turn, metric)| build_turn_analysis(turn, metric, max_context))
        .collect();

    let content = SessionAnalysisViewModel {
        header,
        context_summary,
        turns,
    };

    CommandResultViewModel::new(content).with_badge(StatusBadge::success("Session Analysis"))
}

fn build_context_summary(total_tokens: u32, max_context: Option<u32>) -> ContextWindowSummary {
    let (usage_percent, usage_fraction, progress_bar, warning) = if let Some(max) = max_context {
        let percent = (total_tokens as f64 / max as f64) * 100.0;
        let bar_width = 40;
        let filled = ((percent / 100.0) * bar_width as f64) as usize;
        let filled = filled.min(bar_width);
        let empty = bar_width - filled;

        let bar = format!(
            "[{}{}] {:.1}% Used ({} / {} limit)",
            "█".repeat(filled),
            "░".repeat(empty),
            percent,
            format_tokens(total_tokens as i64),
            format_tokens(max as i64)
        );

        let warning = if percent > 90.0 {
            Some("Warning: Context window nearly full".to_string())
        } else {
            None
        };

        (
            format!("{:.1}%", percent),
            format!(
                "{} / {}",
                format_tokens(total_tokens as i64),
                format_tokens(max as i64)
            ),
            bar,
            warning,
        )
    } else {
        (
            "N/A".to_string(),
            format_tokens(total_tokens as i64),
            format!("Total: {}", format_tokens(total_tokens as i64)),
            None,
        )
    };

    ContextWindowSummary {
        progress_bar,
        usage_percent,
        usage_fraction,
        warning,
    }
}

fn build_turn_analysis(
    turn: &agtrace_engine::AgentTurn,
    metric: &agtrace_engine::TurnMetrics,
    max_context: Option<u32>,
) -> TurnAnalysisViewModel {
    let user_query = truncate_text(&turn.user.content.text, 80);

    let prev_pct = if let Some(max) = max_context {
        format!("{:.1}%", (metric.prev_total as f64 / max as f64) * 100.0)
    } else {
        format_tokens(metric.prev_total as i64)
    };

    let new_pct = if let Some(max) = max_context {
        format!(
            "{:.1}%",
            ((metric.prev_total + metric.delta) as f64 / max as f64) * 100.0
        )
    } else {
        format_tokens((metric.prev_total + metric.delta) as i64)
    };

    let context_transition = format!("{} -> {}", prev_pct, new_pct);

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
                    preview: truncate_text(&reasoning.content.text, 60),
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
                    text: truncate_text(&message.content.text, 80),
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
        timestamp: turn.steps.first().map(|s| format_timestamp(s.timestamp)),
        context_transition,
        context_usage,
        is_heavy_load: metric.is_heavy,
        user_query,
        steps,
        metrics: ViewTurnMetrics {
            total_delta: format!("+{}", format_tokens(metric.delta as i64)),
            input: format_tokens(total_input),
            output: format_tokens(total_output),
            cache_read: if cache_read_total > 0 {
                Some(format_tokens(cache_read_total))
            } else {
                None
            },
        },
    }
}

fn compute_duration(session: &AgentSession) -> Option<String> {
    let start = session.turns.first()?.steps.first()?.timestamp;
    let end = session.turns.last()?.steps.last()?.timestamp;
    let duration = end.signed_duration_since(start);

    let minutes = duration.num_minutes();
    let seconds = duration.num_seconds() % 60;

    if minutes > 0 {
        Some(format!("{}m {}s", minutes, seconds))
    } else {
        Some(format!("{}s", seconds))
    }
}

fn format_timestamp(ts: chrono::DateTime<chrono::Utc>) -> String {
    ts.with_timezone(&chrono::Local)
        .format("%H:%M:%S")
        .to_string()
}

fn format_tokens(count: i64) -> String {
    if count >= 1_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    } else if count >= 1_000 {
        format!("{:.1}k", count as f64 / 1_000.0)
    } else {
        count.to_string()
    }
}

fn truncate_text(text: &str, max_len: usize) -> String {
    if text.chars().count() <= max_len {
        text.to_string()
    } else {
        let truncated: String = text.chars().take(max_len.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
}

fn format_tool_args(tool_call: &agtrace_types::ToolCallPayload) -> String {
    use agtrace_types::ToolCallPayload;

    match tool_call {
        ToolCallPayload::FileRead { arguments, .. } => {
            format_args_compact(&serde_json::to_value(arguments).unwrap_or_default())
        }
        ToolCallPayload::FileEdit { arguments, .. } => {
            format_args_compact(&serde_json::to_value(arguments).unwrap_or_default())
        }
        ToolCallPayload::FileWrite { arguments, .. } => {
            format_args_compact(&serde_json::to_value(arguments).unwrap_or_default())
        }
        ToolCallPayload::Execute { arguments, .. } => {
            format_args_compact(&serde_json::to_value(arguments).unwrap_or_default())
        }
        ToolCallPayload::Search { arguments, .. } => {
            format_args_compact(&serde_json::to_value(arguments).unwrap_or_default())
        }
        ToolCallPayload::Mcp { arguments, .. } => {
            format_args_compact(&serde_json::to_value(arguments).unwrap_or_default())
        }
        ToolCallPayload::Generic { arguments, .. } => {
            format_args_compact(&serde_json::to_value(arguments).unwrap_or_default())
        }
    }
}

fn format_args_compact(args: &serde_json::Value) -> String {
    if let Some(obj) = args.as_object() {
        let pairs: Vec<String> = obj
            .iter()
            .map(|(k, v)| {
                let value_str = match v {
                    serde_json::Value::String(s) => {
                        if s.len() > 50 {
                            format!("\"{}...\"", s.chars().take(47).collect::<String>())
                        } else {
                            format!("\"{}\"", s)
                        }
                    }
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::Bool(b) => b.to_string(),
                    serde_json::Value::Null => "null".to_string(),
                    serde_json::Value::Array(_) => "[...]".to_string(),
                    serde_json::Value::Object(_) => "{...}".to_string(),
                };
                format!("{}: {}", k, value_str)
            })
            .collect();

        if pairs.is_empty() {
            "{}".to_string()
        } else {
            pairs.join(", ")
        }
    } else {
        serde_json::to_string(args).unwrap_or_else(|_| "{}".to_string())
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
        let sample_args = format_tool_args(&tools[0].call.content);
        let has_errors = tools.iter().any(|t| {
            t.result
                .as_ref()
                .map(|r| r.content.is_error)
                .unwrap_or(false)
        });

        vec![AgentStepViewModel::ToolCallSequence {
            name,
            count: tools.len(),
            sample_args,
            has_errors,
        }]
    } else {
        // Otherwise, create individual ToolCall entries
        tools
            .iter()
            .map(|tool| {
                let name = tool.call.content.name().to_string();
                let args = format_tool_args(&tool.call.content);
                let is_error = tool
                    .result
                    .as_ref()
                    .map(|r| r.content.is_error)
                    .unwrap_or(false);
                let result_text = tool
                    .result
                    .as_ref()
                    .map(|r| truncate_text(&r.content.output, 60))
                    .unwrap_or_else(|| "(no result)".to_string());

                AgentStepViewModel::ToolCall {
                    name,
                    args,
                    result: result_text,
                    is_error,
                }
            })
            .collect()
    }
}
