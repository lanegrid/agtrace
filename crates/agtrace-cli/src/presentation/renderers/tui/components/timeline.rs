use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use super::Component;
use crate::presentation::renderers::tui::app::AppState;

pub(crate) struct TimelineComponent;

impl Component for TimelineComponent {
    fn render(&self, f: &mut Frame, area: Rect, state: &mut AppState) {
        let items: Vec<ListItem> = state
            .events_buffer
            .iter()
            .map(|line| ListItem::new(Line::from(line.as_str())))
            .collect();

        let events_list = List::new(items)
            .block(Block::default().borders(Borders::NONE))
            .highlight_style(Style::default().bg(Color::DarkGray));

        f.render_stateful_widget(events_list, area, &mut state.list_state);
    }
}
