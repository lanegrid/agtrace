use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::Component;
use crate::presentation::renderers::tui::app::AppState;

pub(crate) struct SessionHeaderComponent;
pub(crate) struct GlobalLifeGaugeComponent;

impl Component for SessionHeaderComponent {
    fn render(&self, f: &mut Frame, area: Rect, state: &mut AppState) {
        render_session_header(f, area, state);
    }
}

impl Component for GlobalLifeGaugeComponent {
    fn render(&self, f: &mut Frame, area: Rect, state: &mut AppState) {
        render_global_life_gauge(f, area, state);
    }
}

fn render_session_header(f: &mut Frame, area: Rect, state: &AppState) {
    let mut spans = Vec::new();

    if let Some(provider) = &state.provider_name {
        spans.push(Span::styled(
            provider.clone(),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(" / "));
    }

    if let Some(session_id) = &state.attached_session_id {
        let short_id = if session_id.len() > 8 {
            &session_id[..8]
        } else {
            session_id
        };
        spans.push(Span::styled(
            short_id.to_string(),
            Style::default().fg(Color::Yellow),
        ));
        spans.push(Span::raw(" / "));
    }

    let status_text = if state.last_activity.is_some() {
        "Active"
    } else {
        "Inactive"
    };
    let status_color = if state.last_activity.is_some() {
        Color::Green
    } else {
        Color::DarkGray
    };
    spans.push(Span::styled(status_text, Style::default().fg(status_color)));

    if spans.is_empty() {
        spans.push(Span::styled(
            "Waiting for session...",
            Style::default().fg(Color::DarkGray),
        ));
    }

    let line = Line::from(spans);
    f.render_widget(Paragraph::new(line), area);
}

fn render_global_life_gauge(f: &mut Frame, area: Rect, state: &AppState) {
    if let Some(ctx) = &state.context_usage {
        let ratio = ctx.used as f64 / ctx.limit as f64;

        let gauge_color = if ratio > 0.9 {
            Color::Red
        } else if ratio > 0.8 {
            Color::Yellow
        } else {
            Color::Cyan
        };

        let bar_width = area.width.saturating_sub(2) as usize;
        let filled = ((ratio * bar_width as f64) as usize).min(bar_width);
        let empty = bar_width.saturating_sub(filled);
        let bar = format!("[{}{}]", "=".repeat(filled), ".".repeat(empty));

        let lines = vec![
            Line::from(vec![Span::styled(
                "LIFE:",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![Span::styled(bar, Style::default().fg(gauge_color))]),
            Line::from(vec![
                Span::styled(
                    format!(
                        "{} / {} ",
                        format_tokens(ctx.used as i32),
                        format_tokens(ctx.limit as i32)
                    ),
                    Style::default().fg(Color::White),
                ),
                Span::styled(
                    format!("({:.1}%)", ratio * 100.0),
                    Style::default().fg(gauge_color),
                ),
            ]),
        ];

        f.render_widget(Paragraph::new(lines), area);
    } else {
        let lines = vec![
            Line::from("LIFE:"),
            Line::from("Waiting for context data..."),
        ];
        f.render_widget(Paragraph::new(lines), area);
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
