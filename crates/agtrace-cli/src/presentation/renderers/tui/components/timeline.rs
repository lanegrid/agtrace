use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List},
    Frame,
};

use super::Component;
use crate::presentation::renderers::tui::app::AppState;

pub(crate) struct TimelineComponent;

impl Component for TimelineComponent {
    fn render(&self, f: &mut Frame, area: Rect, state: &mut AppState) {
        let events_list = List::new(state.timeline_items.clone())
            .block(
                Block::default()
                    .borders(Borders::NONE)
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        f.render_stateful_widget(events_list, area, &mut state.list_state);
    }
}
