//! TUI Presenter for Watch command
//!
//! This module contains PURE FUNCTIONS that convert domain models
//! (SessionState, AgentEvent, AgentSession) into TUI-specific ViewModels.
//!
//! ## Design Principles:
//! - NO state management (Handler owns state, Presenter is stateless)
//! - ALL calculations and logic happen here (colors, widths, truncation)
//! - Views should only need to map data to widgets, NO decisions

use agtrace_sdk::types::SessionAnalysisExt;
use chrono::Utc;
use std::collections::VecDeque;

use crate::presentation::view_models::{
    ChildStreamViewModel, ContextBreakdownViewModel, DashboardViewModel, StatusBarViewModel,
    StepPreviewViewModel, TimelineEventViewModel, TimelineViewModel, TuiScreenViewModel,
    TurnHistoryViewModel, TurnItemViewModel, WaitingKind, WaitingState, common::StatusLevel,
};

/// Build complete screen ViewModel from current domain state
///
/// This is the main entry point called by the Handler.
/// It produces a complete snapshot of what the TUI should display.
pub fn build_screen_view_model(
    state: &agtrace_sdk::types::SessionState,
    events: &VecDeque<agtrace_sdk::types::AgentEvent>,
    assembled_sessions: &[agtrace_sdk::types::AgentSession],
    max_context: Option<u32>,
    notification: Option<&str>,
) -> TuiScreenViewModel {
    let dashboard = build_dashboard(state, notification);
    let timeline = build_timeline(events);
    let turn_history = build_turn_history(state, assembled_sessions, max_context);
    let status_bar = build_status_bar(state, assembled_sessions);

    TuiScreenViewModel {
        dashboard,
        timeline,
        turn_history,
        status_bar,
    }
}

/// Build dashboard ViewModel with context usage calculations
fn build_dashboard(
    state: &agtrace_sdk::types::SessionState,
    notification: Option<&str>,
) -> DashboardViewModel {
    use agtrace_sdk::types::ContextLimit;

    // Same fallback logic as present_session_state: try context_window_limit first, then model lookup
    let token_limits = agtrace_sdk::utils::default_token_limits();
    let token_spec = state.model.as_ref().and_then(|m| token_limits.get_limit(m));
    let limit_u64 = state
        .context_window_limit
        .or_else(|| token_spec.as_ref().map(|spec| spec.effective_limit()));

    let limit_opt = limit_u64.map(ContextLimit::new);
    let total = state.total_tokens();

    let breakdown = ContextBreakdownViewModel {
        fresh_input: state.current_usage.fresh_input.0.max(0) as u64,
        cache_creation: state.current_usage.cache_creation.0.max(0) as u64,
        cache_read: state.current_usage.cache_read.0.max(0) as u64,
        output: state.current_usage.output.0.max(0) as u64,
        total: total.as_u64(),
    };

    // Calculate usage percentage and color only if limit is known
    let (usage_pct, context_color) = if let Some(limit) = limit_opt {
        let pct = limit.usage_ratio(total).min(1.0);
        let color = if limit.is_exceeded(total) || limit.is_danger_zone(total) {
            StatusLevel::Error
        } else if limit.is_warning_zone(total) {
            StatusLevel::Warning
        } else {
            StatusLevel::Success
        };
        (Some(pct), color)
    } else {
        // No limit known - show as warning (unknown state)
        (None, StatusLevel::Warning)
    };

    let elapsed = (state.last_activity - state.start_time).num_seconds();

    DashboardViewModel {
        title: "AGTRACE".to_string(),
        sub_title: notification.map(|s| s.to_string()),
        session_id: state.session_id.clone(),
        project_root: state.project_root.as_ref().map(|p| p.display().to_string()),
        log_path: state.log_path.as_ref().map(|p| p.display().to_string()),
        model: state.model.clone(),
        start_time: state.start_time,
        last_activity: state.last_activity,
        elapsed_seconds: elapsed.max(0) as u64,
        context_total: total.as_u64(),
        context_limit: limit_opt.map(|l| l.as_u64()),
        context_usage_pct: usage_pct,
        context_color,
        context_breakdown: breakdown,
    }
}

/// Build timeline ViewModel from recent events
fn build_timeline(events: &VecDeque<agtrace_sdk::types::AgentEvent>) -> TimelineViewModel {
    const MAX_TIMELINE_ITEMS: usize = 50;

    let total_count = events.len();
    let displayed = events
        .iter()
        .rev()
        .take(MAX_TIMELINE_ITEMS)
        .map(event_to_timeline_item)
        .collect::<Vec<_>>();

    TimelineViewModel {
        events: displayed.clone(),
        total_count,
        displayed_count: displayed.len(),
    }
}

/// Convert a single event to timeline item
fn event_to_timeline_item(event: &agtrace_sdk::types::AgentEvent) -> TimelineEventViewModel {
    use agtrace_sdk::types::EventPayload;

    let now = Utc::now();
    let relative_time = format_relative_time(event.timestamp, now);

    let (icon, description, level) = match &event.payload {
        EventPayload::User(content) => {
            // Check for interrupt marker
            if content.text.starts_with("[Request interrupted") {
                (
                    "⚠️".to_string(),
                    "Interrupted by user".to_string(),
                    StatusLevel::Warning,
                )
            } else {
                let preview = truncate_text(&content.text, 150);
                (
                    "👤".to_string(),
                    format!("User: {}", preview),
                    StatusLevel::Info,
                )
            }
        }
        EventPayload::Reasoning(content) => {
            let preview = truncate_text(&content.text, 150);
            (
                "🤔".to_string(),
                format!("Reasoning: {}", preview),
                StatusLevel::Info,
            )
        }
        EventPayload::ToolCall(tool_call) => {
            let name = tool_call.name();
            (
                "🔧".to_string(),
                format!("Tool: {}", name),
                StatusLevel::Success,
            )
        }
        EventPayload::ToolResult(tool_result) => {
            let id = &tool_result.tool_call_id;
            (
                "✅".to_string(),
                format!("Result: {}", id),
                StatusLevel::Success,
            )
        }
        EventPayload::Message(content) => {
            let preview = truncate_text(&content.text, 150);
            (
                "💬".to_string(),
                format!("Message: {}", preview),
                StatusLevel::Info,
            )
        }
        EventPayload::TokenUsage(_) => (
            "📊".to_string(),
            "Token Usage".to_string(),
            StatusLevel::Info,
        ),
        EventPayload::Notification(notification) => {
            let preview = truncate_text(&notification.text, 150);
            (
                "🔔".to_string(),
                format!("Notification: {}", preview),
                StatusLevel::Info,
            )
        }
        EventPayload::SlashCommand(cmd) => (
            "⚡".to_string(),
            format!("Command: {}", cmd.name),
            StatusLevel::Info,
        ),
    };

    TimelineEventViewModel {
        timestamp: event.timestamp,
        relative_time,
        icon,
        description,
        level,
    }
}

/// Build turn history ViewModel from assembled sessions (main + child streams)
fn build_turn_history(
    state: &agtrace_sdk::types::SessionState,
    assembled_sessions: &[agtrace_sdk::types::AgentSession],
    max_context: Option<u32>,
) -> TurnHistoryViewModel {
    use agtrace_sdk::types::StreamId;

    // Find main session (stream_id == Main)
    let main_session = assembled_sessions
        .iter()
        .find(|s| matches!(s.stream_id, StreamId::Main));

    // Detect waiting state and provide contextual information
    let waiting_state = detect_waiting_state(state, main_session, max_context);

    let Some(session) = main_session else {
        return TurnHistoryViewModel {
            turns: Vec::new(),
            active_turn_index: None,
            waiting_state,
        };
    };

    // max_context should come from the handler (already has model fallback logic)
    // If still None, we can't calculate usage bars - return empty
    let Some(max_context_u32) = max_context else {
        return TurnHistoryViewModel {
            turns: Vec::new(),
            active_turn_index: None,
            waiting_state,
        };
    };

    // Collect child sessions (stream_id != Main)
    let child_sessions: Vec<_> = assembled_sessions
        .iter()
        .filter(|s| !matches!(s.stream_id, StreamId::Main))
        .collect();

    let metrics = session.compute_turn_metrics(max_context);
    let active_turn_index = metrics
        .iter()
        .enumerate()
        .find(|(_, m)| m.is_active)
        .map(|(i, _)| i);

    let max_context_u64 = max_context_u32 as u64;

    let turns = session
        .turns
        .iter()
        .zip(metrics.iter())
        .map(|(turn, metric)| {
            // Find child sessions spawned at this turn
            let children_for_turn: Vec<_> = child_sessions
                .iter()
                .filter(|child| {
                    child
                        .spawned_by
                        .as_ref()
                        .is_some_and(|ctx| ctx.turn_index == metric.turn_index)
                })
                .map(|child| build_child_stream_view_model(child, max_context))
                .collect();

            build_turn_item_with_children(turn, metric, max_context_u64, children_for_turn)
        })
        .collect();

    TurnHistoryViewModel {
        turns,
        active_turn_index,
        waiting_state: None, // No waiting state when we have turns
    }
}

/// Detect waiting state and provide contextual information
fn detect_waiting_state(
    state: &agtrace_sdk::types::SessionState,
    assembled_session: Option<&agtrace_sdk::types::AgentSession>,
    max_context: Option<u32>,
) -> Option<WaitingState> {
    let is_waiting_session = state.session_id == "waiting";
    let has_events = state.event_count > 0;

    // Calculate relative time for last activity
    let last_activity_relative = if has_events {
        let now = chrono::Utc::now();
        Some(format_relative_time(state.last_activity, now))
    } else {
        None
    };

    match (
        is_waiting_session,
        assembled_session,
        max_context,
        has_events,
    ) {
        // No session detected yet
        (true, _, _, _) => Some(WaitingState {
            kind: WaitingKind::NoSession,
            session_id: None,
            project_root: state.project_root.as_ref().map(|p| p.display().to_string()),
            event_count: None,
            last_activity_relative: None,
        }),
        // Session detected but not assembled yet (analyzing)
        (false, None, _, true) => Some(WaitingState {
            kind: WaitingKind::Analyzing,
            session_id: Some(state.session_id.chars().take(8).collect()),
            project_root: state.project_root.as_ref().map(|p| p.display().to_string()),
            event_count: Some(state.event_count),
            last_activity_relative,
        }),
        // Max context unknown (rare)
        (false, Some(_), None, _) => Some(WaitingState {
            kind: WaitingKind::MissingContext,
            session_id: Some(state.session_id.chars().take(8).collect()),
            project_root: state.project_root.as_ref().map(|p| p.display().to_string()),
            event_count: Some(state.event_count),
            last_activity_relative,
        }),
        // No waiting state - we have complete data
        _ => None,
    }
}

/// Build a single turn item with computed metrics and child streams
fn build_turn_item_with_children(
    turn: &agtrace_sdk::types::AgentTurn,
    metric: &agtrace_sdk::types::TurnMetrics,
    max_context: u64,
    child_streams: Vec<ChildStreamViewModel>,
) -> TurnItemViewModel {
    let title = truncate_text(&turn.user.content.text, 120);
    let slash_command = turn.user.slash_command.as_ref().map(|cmd| cmd.name.clone());

    // Logic: Calculate bar width based on v1's algorithm
    let max_bar_width = 20;

    // Calculate prev_total and delta as absolute ratios against max_context (v1 logic)
    let prev_ratio = metric.prev_total as f64 / max_context as f64;
    let delta_ratio = metric.delta as f64 / max_context as f64;

    let mut prev_chars = (prev_ratio * max_bar_width as f64) as usize;
    let mut delta_chars = (delta_ratio * max_bar_width as f64) as usize;

    // Ensure at least 1 char for delta if delta > 0 (v1 logic)
    if metric.delta > 0 && delta_chars == 0 {
        delta_chars = 1;
    }

    // Clamp total to bar_width (v1 logic with scale down)
    let total_chars = prev_chars + delta_chars;
    if total_chars > max_bar_width {
        // Scale down proportionally
        let scale = max_bar_width as f64 / total_chars as f64;
        prev_chars = (prev_chars as f64 * scale) as usize;
        delta_chars = max_bar_width.saturating_sub(prev_chars);
    }

    let prev_bar_width = prev_chars as u16;
    let bar_width = (prev_chars + delta_chars) as u16;

    // Output ratios (for potential future use in View)
    let usage_ratio = (metric.prev_total + metric.delta) as f64 / max_context as f64;
    let prev_ratio = metric.prev_total as f64 / max_context as f64;
    let delta_ratio = metric.delta as f64 / max_context as f64;

    // Logic: Determine color based on delta magnitude
    let delta_color = if metric.is_heavy {
        StatusLevel::Warning
    } else {
        StatusLevel::Success
    };

    // Build step preview for active turn
    let recent_steps = if metric.is_active {
        turn.steps
            .iter()
            .rev()
            .take(5)
            .rev()
            .map(build_step_preview)
            .collect()
    } else {
        Vec::new()
    };

    let start_time = turn.steps.first().map(|s| s.timestamp);

    TurnItemViewModel {
        turn_id: metric.turn_index + 1,
        title,
        slash_command,
        is_active: metric.is_active,
        is_heavy: metric.is_heavy,
        prev_total: metric.prev_total,
        delta_tokens: metric.delta,
        usage_ratio,
        prev_ratio,
        delta_ratio,
        bar_width,
        prev_bar_width,
        delta_color,
        recent_steps,
        start_time,
        child_streams,
    }
}

/// Build ChildStreamViewModel from a child session
fn build_child_stream_view_model(
    child: &agtrace_sdk::types::AgentSession,
    max_context: Option<u32>,
) -> ChildStreamViewModel {
    use agtrace_sdk::types::StreamId;

    // Build stream label from stream_id
    let stream_label = match &child.stream_id {
        StreamId::Main => "main".to_string(),
        StreamId::Sidechain { agent_id } => {
            format!("sidechain:{}", &agent_id[..8.min(agent_id.len())])
        }
        StreamId::Subagent { name } => format!("subagent:{}", name),
    };

    // Get first user message
    let first_message = child
        .turns
        .first()
        .map(|t| truncate_text(&t.user.content.text, 60))
        .unwrap_or_else(|| "(empty)".to_string());

    // Build last turn's context bar (only last turn visible)
    let last_turn = child.turns.last().and_then(|last_turn| {
        max_context.and_then(|max_ctx| {
            child
                .compute_turn_metrics(Some(max_ctx))
                .last()
                .map(|last_metric| {
                    Box::new(build_turn_item_with_children(
                        last_turn,
                        last_metric,
                        max_ctx as u64,
                        Vec::new(), // Child streams don't have nested children
                    ))
                })
        })
    });

    // Check if child stream is active (last turn is active)
    let is_active = child
        .turns
        .last()
        .is_some_and(|t| t.steps.iter().any(|s| s.usage.is_none()));

    ChildStreamViewModel {
        stream_label,
        first_message,
        last_turn,
        is_active,
    }
}

/// Build step preview item
fn build_step_preview(step: &agtrace_sdk::types::AgentStep) -> StepPreviewViewModel {
    let (icon, description) = if let Some(reasoning) = &step.reasoning {
        (
            "🤔".to_string(),
            truncate_text(&reasoning.content.text, 100),
        )
    } else if let Some(message) = &step.message {
        ("💬".to_string(), truncate_text(&message.content.text, 100))
    } else if !step.tools.is_empty() {
        build_tool_preview(&step.tools[0])
    } else {
        ("•".to_string(), "Event".to_string())
    };

    let token_usage = step.usage.as_ref().map(|u| u.input_tokens() as u32);

    StepPreviewViewModel {
        timestamp: step.timestamp,
        icon,
        description,
        token_usage,
    }
}

/// Build preview for tool execution with special handling for interaction tools
fn build_tool_preview(tool: &agtrace_sdk::types::ToolExecution) -> (String, String) {
    let tool_name = tool.call.content.name();

    match tool_name {
        "TodoWrite" => {
            let description = extract_todo_preview(&tool.call.content);
            ("📋".to_string(), description)
        }
        "AskUserQuestion" => {
            // Check if we have a user answer in the result
            if let Some(ref result) = tool.result
                && result.content.output.starts_with("User has answered")
            {
                let answer = extract_user_answer(&result.content.output);
                return ("✅".to_string(), format!("Answered: {}", answer));
            }
            // No answer yet - show the question
            let question = extract_question_preview(&tool.call.content);
            ("❓".to_string(), question)
        }
        "Task" => {
            let description = extract_task_preview(&tool.call.content);
            ("🚀".to_string(), description)
        }
        _ => ("🔧".to_string(), format!("Tool: {}", tool_name)),
    }
}

/// Extract todo summary from TodoWrite tool
fn extract_todo_preview(payload: &agtrace_sdk::types::ToolCallPayload) -> String {
    use agtrace_sdk::types::ToolCallPayload;

    if let ToolCallPayload::Generic { arguments, .. } = payload
        && let Some(todos) = arguments.get("todos").and_then(|v| v.as_array())
    {
        let total = todos.len();
        let in_progress = todos
            .iter()
            .filter(|t| t.get("status").and_then(|s| s.as_str()) == Some("in_progress"))
            .count();
        let completed = todos
            .iter()
            .filter(|t| t.get("status").and_then(|s| s.as_str()) == Some("completed"))
            .count();

        if in_progress > 0
            && let Some(active) = todos
                .iter()
                .find(|t| t.get("status").and_then(|s| s.as_str()) == Some("in_progress"))
            && let Some(content) = active.get("content").and_then(|c| c.as_str())
        {
            return format!(
                "{} ({}/{} done)",
                truncate_text(content, 50),
                completed,
                total
            );
        }
        return format!("Plan: {} tasks ({} done)", total, completed);
    }
    "Plan updated".to_string()
}

/// Extract question header from AskUserQuestion tool
fn extract_question_preview(payload: &agtrace_sdk::types::ToolCallPayload) -> String {
    use agtrace_sdk::types::ToolCallPayload;

    if let ToolCallPayload::Generic { arguments, .. } = payload
        && let Some(questions) = arguments.get("questions").and_then(|v| v.as_array())
        && let Some(first) = questions.first()
        && let Some(header) = first.get("header").and_then(|h| h.as_str())
    {
        if let Some(question) = first.get("question").and_then(|q| q.as_str()) {
            return format!("[{}] {}", header, truncate_text(question, 60));
        }
        return format!("Asked: {}", header);
    }
    "Asked user".to_string()
}

/// Extract user answer from AskUserQuestion result
fn extract_user_answer(output: &str) -> String {
    // Format: "User has answered your questions: \"question\"=\"answer\", ..."
    if let Some(start) = output.find("=\"") {
        let rest = &output[start + 2..];
        if let Some(end) = rest.find('"') {
            return truncate_text(&rest[..end], 60);
        }
    }
    truncate_text(output, 60)
}

/// Extract Task agent description
fn extract_task_preview(payload: &agtrace_sdk::types::ToolCallPayload) -> String {
    use agtrace_sdk::types::ToolCallPayload;

    if let ToolCallPayload::Generic { arguments, .. } = payload {
        if let Some(desc) = arguments.get("description").and_then(|d| d.as_str()) {
            return format!("Agent: {}", truncate_text(desc, 50));
        }
        if let Some(prompt) = arguments.get("prompt").and_then(|p| p.as_str()) {
            return format!("Agent: {}", truncate_text(prompt, 50));
        }
    }
    "Spawned agent".to_string()
}

/// Build status bar ViewModel
fn build_status_bar(
    state: &agtrace_sdk::types::SessionState,
    assembled_sessions: &[agtrace_sdk::types::AgentSession],
) -> StatusBarViewModel {
    use agtrace_sdk::types::StreamId;

    let session_preview: String = state.session_id.chars().take(8).collect();
    let status_message = format!("Watching session {}...", session_preview);
    let status_level = StatusLevel::Info;

    // Get turn count from main session (consistent with SATURATION HISTORY)
    // This ensures status bar shows the same turn count as the turn history panel
    let main_session_turn_count = assembled_sessions
        .iter()
        .find(|s| matches!(s.stream_id, StreamId::Main))
        .map(|s| s.turns.len())
        .unwrap_or(0);

    StatusBarViewModel {
        event_count: state.event_count,
        turn_count: main_session_turn_count,
        status_message,
        status_level,
    }
}

// --------------------------------------------------------
// Utility Functions (Text Processing)
// --------------------------------------------------------

/// Truncate text to max length with ellipsis
fn truncate_text(text: &str, max_len: usize) -> String {
    if text.chars().count() <= max_len {
        text.to_string()
    } else {
        let truncated: String = text.chars().take(max_len.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
}

/// Format relative time (e.g., "2s ago", "5m ago")
fn format_relative_time(timestamp: chrono::DateTime<Utc>, now: chrono::DateTime<Utc>) -> String {
    let duration = now.signed_duration_since(timestamp);
    let seconds = duration.num_seconds();

    if seconds < 60 {
        format!("{}s ago", seconds)
    } else if seconds < 3600 {
        format!("{}m ago", seconds / 60)
    } else if seconds < 86400 {
        format!("{}h ago", seconds / 3600)
    } else {
        format!("{}d ago", seconds / 86400)
    }
}
