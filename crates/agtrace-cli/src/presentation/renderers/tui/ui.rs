use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Text},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::app::AppState;
use super::components::{Component, DashboardComponent, TimelineComponent};

pub(crate) fn draw(f: &mut Frame, state: &AppState) {
    let has_context = state.context_usage.is_some();
    let dashboard_height = if has_context { 2 } else { 1 };
    let footer_height = state.footer_lines.len().max(1) as u16;

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(dashboard_height),
            Constraint::Min(0),
            Constraint::Length(footer_height),
        ])
        .split(f.area());

    let dashboard = DashboardComponent;
    dashboard.render(f, main_chunks[0], state);

    let timeline = TimelineComponent;
    timeline.render(f, main_chunks[1], state);

    render_footer(f, main_chunks[2], state);
}

fn render_footer(f: &mut Frame, area: Rect, state: &AppState) {
    let footer_text: Vec<Line> = state
        .footer_lines
        .iter()
        .map(|line| Line::from(line.as_str()))
        .collect();

    let footer_widget = Paragraph::new(Text::from(footer_text)).block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    f.render_widget(footer_widget, area);
}

use ratatui::layout::Rect;
