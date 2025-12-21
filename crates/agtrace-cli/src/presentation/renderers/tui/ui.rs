use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};

use super::app::AppState;
use super::components::{Component, DashboardComponent, FooterComponent, TimelineComponent};

pub(crate) fn draw(f: &mut Frame, state: &mut AppState) {
    let has_context = state.context_usage.is_some();
    let has_session = state.attached_session_id.is_some();

    let mut dashboard_height = 1; // Title bar
    if has_session {
        dashboard_height += 1; // Status line
    }
    if has_context {
        dashboard_height += 3; // Context gauge + 2 lines of details
    }

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
