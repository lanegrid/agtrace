use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};

use super::app::AppState;
use super::components::{
    Component, GlobalLifeGaugeComponent, SessionHeaderComponent, TurnHistoryComponent,
};

pub(crate) fn draw(f: &mut Frame, state: &mut AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Session Header with border
            Constraint::Length(4), // Global Life Gauge with border
            Constraint::Min(0),    // Consumption History (remaining)
        ])
        .split(f.area());

    let session_header = SessionHeaderComponent;
    session_header.render(f, chunks[0], state);

    let life_gauge = GlobalLifeGaugeComponent;
    life_gauge.render(f, chunks[1], state);

    let turn_history = TurnHistoryComponent;
    turn_history.render(f, chunks[2], state);
}
