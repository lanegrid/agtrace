//! Turn History View Component
//!
//! Renders the left sidebar with turn list and delta visualization.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Widget},
};

use crate::presentation::v2::view_models::TurnHistoryViewModel;

use super::status_level_to_color;

/// Turn history view wrapper
pub struct TurnHistoryView<'a> {
    model: &'a TurnHistoryViewModel,
}

impl<'a> TurnHistoryView<'a> {
    pub fn new(model: &'a TurnHistoryViewModel) -> Self {
        Self { model }
    }
}

impl<'a> Widget for TurnHistoryView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.model.turns.is_empty() {
            let block = Block::default().title("Turns").borders(Borders::ALL);
            let empty = Paragraph::new("No turns yet...").block(block);
            empty.render(area, buf);
            return;
        }

        let block = Block::default()
            .title(format!("Turns ({})", self.model.turns.len()))
            .borders(Borders::ALL);

        let inner = block.inner(area);
        block.render(area, buf);

        // Split into turn list and active turn detail
        if self.model.active_turn_index.is_some() {
            let chunks = Layout::vertical([Constraint::Percentage(60), Constraint::Percentage(40)])
                .split(inner);

            // Render turn list
            self.render_turn_list(chunks[0], buf);

            // Render active turn details if present
            if let Some(idx) = self.model.active_turn_index {
                if let Some(turn) = self.model.turns.get(idx) {
                    self.render_active_turn_detail(chunks[1], buf, turn);
                }
            }
        } else {
            // Just render turn list
            self.render_turn_list(inner, buf);
        }
    }
}

impl<'a> TurnHistoryView<'a> {
    fn render_turn_list(&self, area: Rect, buf: &mut Buffer) {
        let items: Vec<ListItem> = self
            .model
            .turns
            .iter()
            .map(|turn| {
                let mut line_spans = vec![Span::raw(format!("#{} ", turn.turn_id))];

                // Delta bar (pre-computed width and color)
                let bar_char = if turn.is_heavy { "█" } else { "▓" };
                let bar = bar_char.repeat(turn.delta_bar_width as usize);
                let bar_color = status_level_to_color(turn.delta_color);

                line_spans.push(Span::styled(bar, Style::default().fg(bar_color)));
                line_spans.push(Span::raw(" "));

                // Title with indicator for active turn
                if turn.is_active {
                    line_spans.push(Span::styled(
                        format!("▶ {}", turn.title),
                        Style::default().add_modifier(Modifier::BOLD),
                    ));
                } else {
                    line_spans.push(Span::raw(&turn.title));
                }

                ListItem::new(Line::from(line_spans))
            })
            .collect();

        let list = List::new(items);
        list.render(area, buf);
    }

    fn render_active_turn_detail(
        &self,
        area: Rect,
        buf: &mut Buffer,
        turn: &crate::presentation::v2::view_models::TurnItemViewModel,
    ) {
        let block = Block::default()
            .title("Active Turn Steps")
            .borders(Borders::TOP);

        let inner = block.inner(area);
        block.render(area, buf);

        if turn.recent_steps.is_empty() {
            let empty = Paragraph::new("No steps yet...");
            empty.render(inner, buf);
            return;
        }

        let lines: Vec<Line> = turn
            .recent_steps
            .iter()
            .map(|step| {
                let mut spans = vec![
                    Span::raw(format!("{} ", step.icon)),
                    Span::raw(&step.description),
                ];

                if let Some(tokens) = step.token_usage {
                    spans.push(Span::styled(
                        format!(" ({}t)", tokens),
                        Style::default().add_modifier(Modifier::DIM),
                    ));
                }

                Line::from(spans)
            })
            .collect();

        let paragraph = Paragraph::new(lines);
        paragraph.render(inner, buf);
    }
}
