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
        ])
        .split(f.area());

    let dashboard = DashboardComponent;
    dashboard.render(f, chunks[0], state);

    let turn_history = TurnHistoryComponent;
    turn_history.render(f, chunks[1], state);
}
