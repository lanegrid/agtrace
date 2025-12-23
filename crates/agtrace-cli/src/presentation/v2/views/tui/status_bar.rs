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
        use ratatui::layout::{Constraint, Layout};

        let color = status_level_to_color(self.model.status_level);

        let block = Block::default().borders(Borders::ALL);
        let inner = block.inner(area);
        block.render(area, buf);

        // Split into left (status) and right (help) sections
        let chunks = Layout::horizontal([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(inner);

        // Left: Status information
        let status_line = Line::from(vec![
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
        let status_paragraph = Paragraph::new(status_line);
        status_paragraph.render(chunks[0], buf);

        // Right: Keyboard shortcuts (v1-style)
        let help_line = Line::from(vec![
            Span::styled("[q]", Style::default().fg(ratatui::style::Color::Yellow)),
            Span::raw("uit "),
            Span::styled("[j/k]", Style::default().fg(ratatui::style::Color::Yellow)),
            Span::raw("scroll "),
            Span::styled("[↑/↓]", Style::default().fg(ratatui::style::Color::Yellow)),
            Span::raw("scroll"),
        ]);
        let help_paragraph = Paragraph::new(help_line);
        help_paragraph.render(chunks[1], buf);
    }
}
