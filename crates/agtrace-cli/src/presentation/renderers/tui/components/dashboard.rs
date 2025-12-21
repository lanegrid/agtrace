use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Gauge, Paragraph},
    Frame,
};

use super::Component;
use crate::presentation::renderers::tui::app::{AppState, WatchMode};

pub(crate) struct DashboardComponent;

impl Component for DashboardComponent {
    fn render(&self, f: &mut Frame, area: Rect, state: &mut AppState) {
        let has_context = state.context_usage.is_some();
        let has_session = state.attached_session_id.is_some();

        let mut constraints = vec![Constraint::Length(1)]; // Title bar

        if has_session {
            constraints.push(Constraint::Length(1)); // Status line
        }

        if has_context {
            constraints.push(Constraint::Length(1)); // Context gauge
            constraints.push(Constraint::Length(2)); // Context details
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);

        let mut chunk_idx = 0;
        render_title_bar(f, chunks[chunk_idx], state);
        chunk_idx += 1;

        if has_session {
            render_status_line(f, chunks[chunk_idx], state);
            chunk_idx += 1;
        }

        if has_context {
            render_context_bar(f, chunks[chunk_idx], state);
            chunk_idx += 1;
            render_context_details(f, chunks[chunk_idx], state);
        }
    }
}

fn render_title_bar(f: &mut Frame, area: Rect, state: &AppState) {
    let title_text = if state.session_title.is_empty() {
        "AgTrace Watch".to_string()
    } else {
        format!("AgTrace Watch: {}", state.session_title)
    };

    let mode_text = match state.mode {
        WatchMode::AutoFollow => "🔴 LIVE",
        WatchMode::Fixed => "⏸️  PAUSED",
    };

    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(area);

    f.render_widget(
        Paragraph::new(title_text).style(Style::default().fg(Color::Cyan)),
        layout[0],
    );
    f.render_widget(
        Paragraph::new(mode_text).alignment(Alignment::Right),
        layout[1],
    );
}

fn render_status_line(f: &mut Frame, area: Rect, state: &AppState) {
    let mut parts = Vec::new();

    if let Some(provider) = &state.provider_name {
        parts.push(format!("Provider: {}", provider));
    }

    if let Some(session_id) = &state.attached_session_id {
        let short_id = if session_id.len() > 8 {
            &session_id[..8]
        } else {
            session_id
        };
        parts.push(format!("Session: {}", short_id));
    }

    if let Some(model) = &state.model {
        parts.push(format!("Model: {}", model));
    }

    let status_text = if parts.is_empty() {
        "Waiting for session...".to_string()
    } else {
        parts.join(" | ")
    };

    f.render_widget(
        Paragraph::new(status_text).style(Style::default().fg(Color::Gray)),
        area,
    );
}

fn render_context_bar(f: &mut Frame, area: Rect, state: &AppState) {
    if let Some(ctx) = &state.context_usage {
        let label = format!("{}/{} tokens", ctx.used, ctx.limit);
        let ratio = ctx.used as f64 / ctx.limit as f64;

        let color = if ratio > 0.9 {
            Color::Red
        } else if ratio > 0.7 {
            Color::Yellow
        } else {
            Color::Green
        };

        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(color))
            .label(label)
            .ratio(ratio.min(1.0));

        f.render_widget(gauge, area);
    }
}

fn render_context_details(f: &mut Frame, area: Rect, state: &AppState) {
    if let Some(ctx) = &state.context_usage {
        let input_total = ctx.fresh_input + ctx.cache_creation + ctx.cache_read;

        let line1 = format!(
            "In: {} (Fresh: {} / Cache: {}+{}) | Out: {}",
            format_tokens(input_total),
            format_tokens(ctx.fresh_input),
            format_tokens(ctx.cache_creation),
            format_tokens(ctx.cache_read),
            format_tokens(ctx.output)
        );

        let line2 = if let Some(buffer_pct) = state.compaction_buffer_pct {
            if buffer_pct > 0.0 {
                let trigger_threshold = 100.0 - buffer_pct;
                let current_input_pct = (input_total as f64 / ctx.limit as f64) * 100.0;

                if current_input_pct >= trigger_threshold {
                    format!("Compaction: TRIGGERED (buffer: {:.0}%)", buffer_pct)
                } else {
                    format!(
                        "Compaction: OK (triggers at {:.0}%, current: {:.1}%)",
                        trigger_threshold, current_input_pct
                    )
                }
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(area);

        f.render_widget(
            Paragraph::new(line1).style(Style::default().fg(Color::DarkGray)),
            chunks[0],
        );

        if !line2.is_empty() {
            let style = if line2.contains("TRIGGERED") {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            f.render_widget(Paragraph::new(line2).style(style), chunks[1]);
        }
    }
}

fn format_tokens(count: i32) -> String {
    if count >= 1_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    } else if count >= 1_000 {
        format!("{:.1}k", count as f64 / 1_000.0)
    } else {
        count.to_string()
    }
}
