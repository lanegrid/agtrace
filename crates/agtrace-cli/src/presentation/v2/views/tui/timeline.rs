//! Timeline View Component
//!
//! Renders the scrollable event timeline.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Widget},
};

use crate::presentation::v2::view_models::TimelineViewModel;

use super::status_level_to_color;

/// Timeline view wrapper
pub struct TimelineView<'a> {
    model: &'a TimelineViewModel,
}

impl<'a> TimelineView<'a> {
    pub fn new(model: &'a TimelineViewModel) -> Self {
        Self { model }
    }
}

impl<'a> Widget for TimelineView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = format!(
            "Timeline ({}/{})",
            self.model.displayed_count, self.model.total_count
        );

        let block = Block::default().title(title.as_str()).borders(Borders::ALL);

        let items: Vec<ListItem> = self
            .model
            .events
            .iter()
            .map(|event| {
                let color = status_level_to_color(event.level);

                let line = Line::from(vec![
                    Span::styled(
                        format!("[{}] ", event.relative_time),
                        Style::default().add_modifier(Modifier::DIM),
                    ),
                    Span::raw(format!("{} ", event.icon)),
                    Span::styled(event.description.as_str(), Style::default().fg(color)),
                ]);

                ListItem::new(line)
            })
            .collect();

        let list = List::new(items).block(block);

        list.render(area, buf);
    }
}
