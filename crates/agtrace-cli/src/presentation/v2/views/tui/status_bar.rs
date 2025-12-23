//! Status Bar View Component
//!
//! Renders the bottom status bar with event/turn counts and status message.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::presentation::v2::view_models::StatusBarViewModel;

use super::status_level_to_color;

/// Status bar view wrapper
pub struct StatusBarView<'a> {
    model: &'a StatusBarViewModel,
}

impl<'a> StatusBarView<'a> {
    pub fn new(model: &'a StatusBarViewModel) -> Self {
        Self { model }
    }
}

impl<'a> Widget for StatusBarView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let color = status_level_to_color(self.model.status_level);

        let line = Line::from(vec![
            Span::styled(
                format!("Events: {} ", self.model.event_count),
                Style::default(),
            ),
            Span::raw("| "),
            Span::styled(
                format!("Turns: {} ", self.model.turn_count),
                Style::default(),
            ),
            Span::raw("| "),
            Span::styled(&self.model.status_message, Style::default().fg(color)),
        ]);

        let block = Block::default().borders(Borders::ALL);
        let paragraph = Paragraph::new(line).block(block);

        paragraph.render(area, buf);
    }
}
