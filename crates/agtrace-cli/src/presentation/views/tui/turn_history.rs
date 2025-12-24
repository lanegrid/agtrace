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

use crate::presentation::view_models::TurnHistoryViewModel;

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
            let block = Block::default()
                .title("SATURATION HISTORY")
                .borders(Borders::ALL);
            let empty = Paragraph::new("Waiting for turn data...").block(block);
            empty.render(area, buf);
            return;
        }

        // v1-style title: "SATURATION HISTORY (Delta Highlight)"
        let block = Block::default()
            .title(format!(
                "SATURATION HISTORY (Delta Highlight) - {} turns",
                self.model.turns.len()
            ))
            .borders(Borders::ALL);

        let inner = block.inner(area);
        block.render(area, buf);

        // Split into turn list and active turn detail (v1 style)
        if self.model.active_turn_index.is_some() {
            let chunks = Layout::vertical([Constraint::Percentage(70), Constraint::Percentage(30)])
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
        let mut items: Vec<ListItem> = Vec::new();

        for (idx, turn) in self.model.turns.iter().enumerate() {
            // v1-style: Add "CURRENT TURN" marker before active turn
            if turn.is_active && idx > 0 {
                items.push(ListItem::new(Line::from(vec![Span::styled(
                    "├─ CURRENT TURN (Active) ────────────────────────────────────",
                    Style::default()
                        .fg(ratatui::style::Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )])));
            }

            let mut line_spans = vec![Span::raw(format!("#{:02} ", turn.turn_id))];

            // Fixed-width stacked bar (v1-style): [history (█) + delta (▓) + void (░)]
            const BAR_WIDTH: usize = 20;
            let prev_chars = turn.prev_bar_width as usize;
            let delta_chars = turn.bar_width.saturating_sub(turn.prev_bar_width) as usize;
            let total_chars = turn.bar_width as usize;
            let remaining_chars = BAR_WIDTH.saturating_sub(total_chars);

            line_spans.push(Span::raw("["));

            // Previous turns (dark gray █)
            if prev_chars > 0 {
                line_spans.push(Span::styled(
                    "█".repeat(prev_chars),
                    Style::default().fg(ratatui::style::Color::DarkGray),
                ));
            }

            // Current turn delta (colored ▓)
            if delta_chars > 0 {
                let delta_color = status_level_to_color(turn.delta_color);
                line_spans.push(Span::styled(
                    "▓".repeat(delta_chars),
                    Style::default()
                        .fg(delta_color)
                        .add_modifier(Modifier::BOLD),
                ));
            }

            // Void/unused portion (dim gray ░)
            if remaining_chars > 0 {
                line_spans.push(Span::styled(
                    "░".repeat(remaining_chars),
                    Style::default().fg(ratatui::style::Color::Rgb(60, 60, 60)),
                ));
            }

            line_spans.push(Span::raw("] "));

            // Token info: percentage and delta tokens (v1-style compact form)
            let pct = turn.usage_ratio * 100.0;
            let pct_color = if turn.is_heavy {
                ratatui::style::Color::Red
            } else {
                ratatui::style::Color::White
            };
            let pct_text = format!("+{:.1}% ({})", pct, format_tokens(turn.delta_tokens));
            line_spans.push(Span::styled(pct_text, Style::default().fg(pct_color)));
            line_spans.push(Span::raw(" "));

            // Title
            if turn.is_active {
                line_spans.push(Span::styled(
                    format!("User: \"{}\"", turn.title),
                    Style::default().add_modifier(Modifier::BOLD),
                ));
            } else {
                line_spans.push(Span::raw(format!("User: \"{}\"", turn.title)));
            }

            items.push(ListItem::new(Line::from(line_spans)));
        }

        let list = List::new(items);
        list.render(area, buf);
    }

    fn render_active_turn_detail(
        &self,
        area: Rect,
        buf: &mut Buffer,
        turn: &crate::presentation::view_models::TurnItemViewModel,
    ) {
        // v1-style: "Recent Steps" (instead of "Active Turn Steps")
        let block = Block::default().title("Recent Steps").borders(Borders::TOP);

        let inner = block.inner(area);
        block.render(area, buf);

        if turn.recent_steps.is_empty() {
            let empty = Paragraph::new("Waiting for steps...");
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
                        format!(" (+{})", format_tokens(tokens)),
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

/// Format token count in compact form (k, M)
fn format_tokens(count: u32) -> String {
    if count >= 1_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    } else if count >= 1_000 {
        format!("{:.1}k", count as f64 / 1_000.0)
    } else {
        count.to_string()
    }
}
