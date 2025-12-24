//! TUI Presenter for Watch command
//!
//! This module contains PURE FUNCTIONS that convert domain models
//! (SessionState, AgentEvent, AgentSession) into TUI-specific ViewModels.
//!
//! ## Design Principles:
//! - NO state management (Handler owns state, Presenter is stateless)
//! - ALL calculations and logic happen here (colors, widths, truncation)
//! - Views should only need to map data to widgets, NO decisions

use chrono::Utc;
use std::collections::VecDeque;

use crate::presentation::view_models::{
    ContextBreakdownViewModel, DashboardViewModel, StatusBarViewModel, StepPreviewViewModel,
    TimelineEventViewModel, TimelineViewModel, TuiScreenViewModel, TurnHistoryViewModel,
    TurnItemViewModel, common::StatusLevel,
};

/// Build complete screen ViewModel from current domain state
///
/// This is the main entry point called by the Handler.
/// It produces a complete snapshot of what the TUI should display.
pub fn build_screen_view_model(
    state: &agtrace_runtime::SessionState,
    events: &VecDeque<agtrace_types::AgentEvent>,
    assembled_session: Option<&agtrace_engine::AgentSession>,
    max_context: Option<u32>,
    notification: Option<&str>,
) -> TuiScreenViewModel {
    let dashboard = build_dashboard(state, notification);
    let timeline = build_timeline(events);
    let turn_history = build_turn_history(assembled_session, max_context);
    let status_bar = build_status_bar(state);

    TuiScreenViewModel {
        dashboard,
        timeline,
        turn_history,
        status_bar,
    }
}

/// Build dashboard ViewModel with context usage calculations
fn build_dashboard(
    state: &agtrace_runtime::SessionState,
    notification: Option<&str>,
) -> DashboardViewModel {
    use agtrace_engine::ContextLimit;

    // Same fallback logic as present_session_state: try context_window_limit first, then model lookup
    let token_limits = agtrace_runtime::TokenLimits::new();
    let token_spec = state.model.as_ref().and_then(|m| token_limits.get_limit(m));
    let limit_u64 = state
        .context_window_limit
        .or_else(|| token_spec.as_ref().map(|spec| spec.effective_limit()));

    let limit_opt = limit_u64.map(ContextLimit::new);
    let total = state.total_tokens();

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
        model: state.model.clone(),
        start_time: state.start_time,
        last_activity: state.last_activity,
        elapsed_seconds: elapsed.max(0) as u64,
        context_total: total.as_u64(),
        context_limit: limit_opt.map(|l| l.as_u64()),
        context_usage_pct: usage_pct,
        context_color,
        context_breakdown: ContextBreakdownViewModel {
            fresh_input: state.current_usage.fresh_input.0.max(0) as u64,
            cache_creation: state.current_usage.cache_creation.0.max(0) as u64,
            cache_read: state.current_usage.cache_read.0.max(0) as u64,
            output: state.current_usage.output.0.max(0) as u64,
            total: total.as_u64(),
        },
    }
}

/// Build timeline ViewModel from recent events
fn build_timeline(events: &VecDeque<agtrace_types::AgentEvent>) -> TimelineViewModel {
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
fn event_to_timeline_item(event: &agtrace_types::AgentEvent) -> TimelineEventViewModel {
    use agtrace_types::EventPayload;

    let now = Utc::now();
    let relative_time = format_relative_time(event.timestamp, now);

    let (icon, description, level) = match &event.payload {
        EventPayload::User(content) => {
            let preview = truncate_text(&content.text, 150);
            (
                "👤".to_string(),
                format!("User: {}", preview),
                StatusLevel::Info,
            )
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
    };

    TimelineEventViewModel {
        timestamp: event.timestamp,
        relative_time,
        icon,
        description,
        level,
    }
}

/// Build turn history ViewModel from assembled session
fn build_turn_history(
    assembled_session: Option<&agtrace_engine::AgentSession>,
    max_context: Option<u32>,
) -> TurnHistoryViewModel {
    let Some(session) = assembled_session else {
        return TurnHistoryViewModel {
            turns: Vec::new(),
            active_turn_index: None,
        };
    };

    // max_context should come from the handler (already has model fallback logic)
    // If still None, we can't calculate usage bars - return empty
    let Some(max_context_u32) = max_context else {
        return TurnHistoryViewModel {
            turns: Vec::new(),
            active_turn_index: None,
        };
    };

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
        .map(|(turn, metric)| build_turn_item(turn, metric, max_context_u64))
        .collect();

    TurnHistoryViewModel {
        turns,
        active_turn_index,
    }
}

/// Build a single turn item with computed metrics
fn build_turn_item(
    turn: &agtrace_engine::AgentTurn,
    metric: &agtrace_engine::TurnMetrics,
    max_context: u64,
) -> TurnItemViewModel {
    let title = truncate_text(&turn.user.content.text, 120);

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
        is_active: metric.is_active,
        is_heavy: metric.is_heavy,
        prev_total: metric.prev_total,
        delta_tokens: metric.delta,
        usage_ratio,
        prev_ratio,
        bar_width,
        prev_bar_width,
        delta_color,
        recent_steps,
        start_time,
    }
}

/// Build step preview item
fn build_step_preview(step: &agtrace_engine::AgentStep) -> StepPreviewViewModel {
    let (icon, description) = if let Some(reasoning) = &step.reasoning {
        (
            "🤔".to_string(),
            truncate_text(&reasoning.content.text, 100),
        )
    } else if let Some(message) = &step.message {
        ("💬".to_string(), truncate_text(&message.content.text, 100))
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

    StepPreviewViewModel {
        timestamp: step.timestamp,
        icon,
        description,
        token_usage,
    }
}

/// Build status bar ViewModel
fn build_status_bar(state: &agtrace_runtime::SessionState) -> StatusBarViewModel {
    let status_message = format!("Watching session {}...", &state.session_id[..8]);
    let status_level = StatusLevel::Info;

    StatusBarViewModel {
        event_count: state.event_count,
        turn_count: state.turn_count,
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
