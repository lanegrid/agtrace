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
            Constraint::Length(1), // Session Header (1-2 lines)
            Constraint::Length(3), // Global Life Gauge (3 lines)
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
