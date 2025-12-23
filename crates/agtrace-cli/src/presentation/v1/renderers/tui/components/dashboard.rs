use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::Component;
use crate::presentation::v1::renderers::tui::app::{AppState, WatchMode};

pub(crate) struct DashboardComponent;

impl Component for DashboardComponent {
    fn render(&self, f: &mut Frame, area: Rect, state: &mut AppState) {
        render_dashboard(f, area, state);
    }
}

fn render_dashboard(f: &mut Frame, area: Rect, state: &AppState) {
    let mut title_spans = vec![Span::styled(
        "AGTRACE",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )];

    if let Some(provider) = &state.provider_name {
        title_spans.push(Span::raw(" :: "));
        title_spans.push(Span::styled(
            provider.clone(),
            Style::default().fg(Color::LightCyan),
        ));
    }

    if let Some(session_id) = &state.attached_session_id {
        let short_id = if session_id.len() > 8 {
            &session_id[..8]
        } else {
            session_id
        };
        title_spans.push(Span::raw(" :: "));
        title_spans.push(Span::styled(
            short_id.to_string(),
            Style::default().fg(Color::Yellow),
        ));
    }

    let mode_text = match state.mode {
        WatchMode::AutoFollow => " [LIVE]",
        WatchMode::Fixed => " [PAUSED]",
    };
    let mode_color = match state.mode {
        WatchMode::AutoFollow => Color::LightRed,
        WatchMode::Fixed => Color::Yellow,
    };
    title_spans.push(Span::raw(" "));
    title_spans.push(Span::styled(
        mode_text,
        Style::default().fg(mode_color).add_modifier(Modifier::BOLD),
    ));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Line::from(title_spans));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if let Some(ctx) = &state.context_usage {
        let ratio = ctx.used as f64 / ctx.limit as f64;

        let gauge_color = if ratio > 0.9 {
            Color::Red
        } else if ratio > 0.8 {
            Color::Yellow
        } else {
            Color::Green
        };

        let bar_width = inner.width.saturating_sub(4) as usize;
        let filled = ((ratio * bar_width as f64) as usize).min(bar_width);
        let empty = bar_width.saturating_sub(filled);
        let bar = format!("[{}{}]", "█".repeat(filled), "░".repeat(empty));

        let lines = vec![
            Line::from(vec![Span::styled(
                format!(
                    "LIFE: {} / {} ({:.0}%)",
                    format_tokens(ctx.used as i32),
                    format_tokens(ctx.limit as i32),
                    ratio * 100.0
                ),
                Style::default().fg(Color::White),
            )]),
            Line::from(vec![Span::styled(bar, Style::default().fg(gauge_color))]),
        ];

        f.render_widget(Paragraph::new(lines), inner);
    } else {
        let lines = vec![
            Line::from("LIFE: Waiting for context data..."),
            Line::from(""),
        ];
        f.render_widget(Paragraph::new(lines), inner);
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
