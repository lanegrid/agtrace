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
        let header_height = if has_context { 2 } else { 1 };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(header_height)])
            .split(area);

        if has_context {
            let inner_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Length(1)])
                .split(chunks[0]);

            render_title_bar(f, inner_chunks[0], state);
            render_context_bar(f, inner_chunks[1], state);
        } else {
            render_title_bar(f, chunks[0], state);
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
