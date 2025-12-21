use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};

use super::app::AppState;
use super::components::{Component, DashboardComponent, TurnHistoryComponent};

pub(crate) fn draw(f: &mut Frame, state: &mut AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Dashboard (Header + Life Gauge) with border
            Constraint::Min(0),    // Turn History (remaining)
            Constraint::Length(1), // Footer (Help)
        ])
        .split(f.area());

    let dashboard = DashboardComponent;
    dashboard.render(f, chunks[0], state);

    let turn_history = TurnHistoryComponent;
    turn_history.render(f, chunks[1], state);

    render_footer(f, chunks[2]);
}

fn render_footer(f: &mut Frame, area: ratatui::layout::Rect) {
    use ratatui::{
        style::{Color, Style},
        text::{Line, Span},
        widgets::Paragraph,
    };

    let footer_line = Line::from(vec![
        Span::styled("[q]", Style::default().fg(Color::Yellow)),
        Span::raw("uit "),
        Span::styled("[j/k]", Style::default().fg(Color::Yellow)),
        Span::raw("scroll "),
        Span::styled("[↑/↓]", Style::default().fg(Color::Yellow)),
        Span::raw("scroll"),
    ]);

    f.render_widget(Paragraph::new(footer_line), area);
}
