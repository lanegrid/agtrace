use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};

use super::app::AppState;
use super::components::{Component, DashboardComponent, FooterComponent, TimelineComponent};

pub(crate) fn draw(f: &mut Frame, state: &mut AppState) {
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

    let footer = FooterComponent;
    footer.render(f, main_chunks[2], state);
}
