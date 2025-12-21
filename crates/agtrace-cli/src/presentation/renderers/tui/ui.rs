use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};

use super::app::AppState;
use super::components::{Component, DashboardComponent, TimelineComponent, TurnHistoryComponent};

pub(crate) fn draw(f: &mut Frame, state: &mut AppState) {
    let has_context = state.context_usage.is_some();
    let has_session = state.attached_session_id.is_some();
    let has_turn_history = !state.turns_usage.is_empty() && state.max_context.is_some();

    let mut dashboard_height = 1; // Title bar
    if has_session {
        dashboard_height += 3; // Status box (with borders)
    }
    if has_context {
        dashboard_height += 6; // Context box (with borders)
    }

    let main_chunks = if has_turn_history {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(dashboard_height),
                Constraint::Percentage(40),
                Constraint::Min(0),
            ])
            .split(f.area());

        let dashboard = DashboardComponent;
        dashboard.render(f, chunks[0], state);

        let turn_history = TurnHistoryComponent;
        turn_history.render(f, chunks[1], state);

        let timeline = TimelineComponent;
        timeline.render(f, chunks[2], state);

        return;
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(dashboard_height), Constraint::Min(0)])
            .split(f.area())
    };

    let dashboard = DashboardComponent;
    dashboard.render(f, main_chunks[0], state);

    let timeline = TimelineComponent;
    timeline.render(f, main_chunks[1], state);
}
