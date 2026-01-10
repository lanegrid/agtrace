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

    /// Build list widget with layout information for stateful rendering.
    ///
    /// Returns (List, Block, inner_area, item_count).
    /// The component uses this to render with ListState for scrolling.
    pub fn build_list_with_layout(&self, area: Rect) -> (List<'a>, Block<'a>, Rect, usize) {
        // Handle waiting state and empty state - return empty list
        if self.model.waiting_state.is_some() || self.model.turns.is_empty() {
            let block = Block::default()
                .title("SATURATION HISTORY")
                .borders(Borders::ALL);
            let inner = block.inner(area);
            return (List::new(Vec::<ListItem>::new()), block, inner, 0);
        }

        // v1-style title
        let block = Block::default()
            .title(format!(
                "SATURATION HISTORY (Delta Highlight) - {} turns",
                self.model.turns.len()
            ))
            .borders(Borders::ALL);

        let inner = block.inner(area);

        // Always show Recent Steps section (70% turns, 30% steps)
        let list_area = {
            let chunks = Layout::vertical([Constraint::Percentage(70), Constraint::Percentage(30)])
                .split(inner);
            chunks[0]
        };

        let (items, item_count) = self.build_list_items();
        let list = List::new(items).highlight_style(
            Style::default()
                // Use a distinct blue-tinted background to avoid collision with history bar (DarkGray)
                .bg(ratatui::style::Color::Rgb(30, 40, 60))
                .add_modifier(Modifier::BOLD),
        );

        (list, block, list_area, item_count)
    }

    /// Build list items for turn history.
    /// Returns (items, total_item_count).
    fn build_list_items(&self) -> (Vec<ListItem<'a>>, usize) {
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

            // Context compaction indicator (if reset occurred)
            if turn.context_compacted {
                line_spans.push(Span::styled(
                    "🔄 ",
                    Style::default().fg(ratatui::style::Color::Magenta),
                ));
            }

            // Token info: delta percentage and delta tokens
            let delta_pct = turn.delta_ratio * 100.0;
            let pct_color = if turn.is_heavy {
                ratatui::style::Color::Red
            } else {
                ratatui::style::Color::White
            };
            let pct_text = format!("+{:.1}% ({})", delta_pct, format_tokens(turn.delta_tokens));
            line_spans.push(Span::styled(pct_text, Style::default().fg(pct_color)));
            line_spans.push(Span::raw(" "));

            // Slash command badge (if present)
            if let Some(ref cmd) = turn.slash_command {
                line_spans.push(Span::styled(
                    format!("[{}] ", cmd),
                    Style::default()
                        .fg(ratatui::style::Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ));
            }

            // Title - special handling for interrupted turns
            let is_interrupted = turn.title.starts_with("[Request interrupted");
            if is_interrupted {
                line_spans.push(Span::styled(
                    "⚠️ INTERRUPTED",
                    Style::default()
                        .fg(ratatui::style::Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ));
            } else if turn.is_active {
                line_spans.push(Span::styled(
                    format!("\"{}\"", turn.title),
                    Style::default().add_modifier(Modifier::BOLD),
                ));
            } else {
                line_spans.push(Span::raw(format!("\"{}\"", turn.title)));
            }

            items.push(ListItem::new(Line::from(line_spans)));

            // Render child streams (indented under parent turn)
            for child in &turn.child_streams {
                items.extend(self.render_child_stream(child));
            }
        }

        let count = items.len();
        (items, count)
    }

    /// Check if there's a waiting state to render.
    pub fn has_waiting_state(&self) -> bool {
        self.model.waiting_state.is_some()
    }

    /// Check if turns are empty.
    pub fn is_empty(&self) -> bool {
        self.model.turns.is_empty()
    }

    /// Render recent steps section (for component use).
    /// Now uses global_recent_steps which shows steps from all turns.
    pub fn render_active_turn_detail_to(&self, f: &mut ratatui::Frame, area: Rect) {
        // v1-style: "Recent Steps" - always visible
        let block = Block::default().title("Recent Steps").borders(Borders::TOP);

        let inner = block.inner(area);
        f.render_widget(block, area);

        if self.model.global_recent_steps.is_empty() {
            let empty = Paragraph::new("Waiting for steps...");
            f.render_widget(empty, inner);
            return;
        }

        let lines: Vec<Line> = self
            .model
            .global_recent_steps
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
        f.render_widget(paragraph, inner);
    }

    /// Get recent steps area (always visible now).
    pub fn get_active_turn_detail_area(&self, inner: Rect) -> Option<Rect> {
        // Always return the area for Recent Steps section
        let chunks = Layout::vertical([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(inner);
        Some(chunks[1])
    }
}

impl<'a> Widget for TurnHistoryView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Show waiting state with helpful hints if present
        if let Some(ref waiting_state) = self.model.waiting_state {
            self.render_waiting_state(area, buf, waiting_state);
            return;
        }

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

        // Always split into turn list and Recent Steps (70/30)
        let chunks = Layout::vertical([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(inner);

        // Render turn list
        self.render_turn_list(chunks[0], buf);

        // Always render Recent Steps (using global_recent_steps)
        self.render_global_recent_steps(chunks[1], buf);
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

            // Context compaction indicator (if reset occurred)
            if turn.context_compacted {
                line_spans.push(Span::styled(
                    "🔄 ",
                    Style::default().fg(ratatui::style::Color::Magenta),
                ));
            }

            // Token info: delta percentage and delta tokens
            let delta_pct = turn.delta_ratio * 100.0;
            let pct_color = if turn.is_heavy {
                ratatui::style::Color::Red
            } else {
                ratatui::style::Color::White
            };
            let pct_text = format!("+{:.1}% ({})", delta_pct, format_tokens(turn.delta_tokens));
            line_spans.push(Span::styled(pct_text, Style::default().fg(pct_color)));
            line_spans.push(Span::raw(" "));

            // Slash command badge (if present)
            if let Some(ref cmd) = turn.slash_command {
                line_spans.push(Span::styled(
                    format!("[{}] ", cmd),
                    Style::default()
                        .fg(ratatui::style::Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ));
            }

            // Title - special handling for interrupted turns
            let is_interrupted = turn.title.starts_with("[Request interrupted");
            if is_interrupted {
                line_spans.push(Span::styled(
                    "⚠️ INTERRUPTED",
                    Style::default()
                        .fg(ratatui::style::Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ));
            } else if turn.is_active {
                line_spans.push(Span::styled(
                    format!("\"{}\"", turn.title),
                    Style::default().add_modifier(Modifier::BOLD),
                ));
            } else {
                line_spans.push(Span::raw(format!("\"{}\"", turn.title)));
            }

            items.push(ListItem::new(Line::from(line_spans)));

            // Render child streams (indented under parent turn)
            for child in &turn.child_streams {
                items.extend(self.render_child_stream(child));
            }
        }

        let list = List::new(items);
        list.render(area, buf);
    }

    /// Render a child stream (sidechain/subagent) with indentation
    fn render_child_stream(
        &self,
        child: &crate::presentation::view_models::ChildStreamViewModel,
    ) -> Vec<ListItem<'a>> {
        use ratatui::style::Color;

        let mut items = Vec::new();

        // First line: stream label with connector
        let label_color = if child.is_active {
            Color::Yellow
        } else {
            Color::Cyan
        };

        let label_line = Line::from(vec![
            Span::raw("  "),
            Span::styled("└ ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                child.stream_label.to_string(),
                Style::default().fg(label_color),
            ),
            Span::raw(" "),
            Span::styled(
                format!("\"{}\"", child.first_message),
                Style::default().fg(Color::DarkGray),
            ),
        ]);
        items.push(ListItem::new(label_line));

        // Second line: last turn's context bar (if available)
        // Use smaller bar width for child streams to visually distinguish from main stream
        if let Some(ref last_turn) = child.last_turn {
            const CHILD_BAR_WIDTH: usize = 12;
            const MAIN_BAR_WIDTH: usize = 20;

            // Scale down from main bar width (20) to child bar width (12)
            let scale = CHILD_BAR_WIDTH as f64 / MAIN_BAR_WIDTH as f64;
            let prev_chars = ((last_turn.prev_bar_width as f64) * scale).round() as usize;
            let total_scaled = ((last_turn.bar_width as f64) * scale).round() as usize;
            let delta_chars = total_scaled.saturating_sub(prev_chars);
            let remaining_chars = CHILD_BAR_WIDTH.saturating_sub(total_scaled);

            let mut bar_spans = vec![
                Span::raw("      "), // 6-space indentation (more than main's implicit 0)
                Span::raw("["),
            ];

            // Previous turns (dark gray █)
            if prev_chars > 0 {
                bar_spans.push(Span::styled(
                    "█".repeat(prev_chars),
                    Style::default().fg(Color::DarkGray),
                ));
            }

            // Current turn delta (colored ▓)
            if delta_chars > 0 {
                let delta_color = status_level_to_color(last_turn.delta_color);
                bar_spans.push(Span::styled(
                    "▓".repeat(delta_chars),
                    Style::default()
                        .fg(delta_color)
                        .add_modifier(Modifier::BOLD),
                ));
            }

            // Void/unused portion (dim gray ░)
            if remaining_chars > 0 {
                bar_spans.push(Span::styled(
                    "░".repeat(remaining_chars),
                    Style::default().fg(Color::Rgb(60, 60, 60)),
                ));
            }

            bar_spans.push(Span::raw("] "));

            // Token info
            let delta_pct = last_turn.delta_ratio * 100.0;
            let pct_color = if last_turn.is_heavy {
                Color::Red
            } else {
                Color::White
            };
            let pct_text = format!(
                "+{:.1}% ({})",
                delta_pct,
                format_tokens(last_turn.delta_tokens)
            );
            bar_spans.push(Span::styled(pct_text, Style::default().fg(pct_color)));

            items.push(ListItem::new(Line::from(bar_spans)));
        }

        items
    }

    /// Render global recent steps (from all turns, not just active turn)
    fn render_global_recent_steps(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::default().title("Recent Steps").borders(Borders::TOP);

        let inner = block.inner(area);
        block.render(area, buf);

        if self.model.global_recent_steps.is_empty() {
            let empty = Paragraph::new("Waiting for steps...");
            empty.render(inner, buf);
            return;
        }

        let lines: Vec<Line> = self
            .model
            .global_recent_steps
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

    /// Render waiting state with contextual hints and commands
    fn render_waiting_state(
        &self,
        area: Rect,
        buf: &mut Buffer,
        waiting_state: &crate::presentation::view_models::WaitingState,
    ) {
        use crate::presentation::view_models::WaitingKind;
        use ratatui::style::Color;

        let block = Block::default()
            .title("SATURATION HISTORY")
            .borders(Borders::ALL);

        let inner = block.inner(area);
        block.render(area, buf);

        let lines = match waiting_state.kind {
            WaitingKind::NoSession => {
                let mut lines = vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        "No active session detected",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                    Line::from("Next steps:"),
                    Line::from("  • Start your AI coding agent (claude, codex, etc.)"),
                    Line::from("  • Or check past sessions:"),
                    Line::from(Span::styled(
                        "    agtrace session list",
                        Style::default().fg(Color::Cyan),
                    )),
                    Line::from(""),
                ];

                if let Some(ref project_root) = waiting_state.project_root {
                    lines.push(Line::from(Span::styled(
                        format!("Monitoring: {}", project_root),
                        Style::default().fg(Color::DarkGray),
                    )));
                    lines.push(Line::from(""));
                }

                lines.push(Line::from(Span::styled(
                    "Note: agtrace requires exact directory match",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(Span::styled(
                    "(not parent or subdirectories)",
                    Style::default().fg(Color::Yellow),
                )));
                lines.push(Line::from(""));

                lines.push(Line::from(Span::styled(
                    "Waiting for new session...",
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::DIM),
                )));

                lines
            }
            WaitingKind::Analyzing => {
                let mut lines = vec![Line::from("")];

                if let Some(ref session_id) = waiting_state.session_id {
                    lines.push(Line::from(vec![
                        Span::raw("Session: "),
                        Span::styled(
                            session_id.clone(),
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]));
                    lines.push(Line::from(""));
                }

                lines.push(Line::from(Span::styled(
                    "Analyzing session data...",
                    Style::default().fg(Color::Yellow),
                )));

                if let Some(count) = waiting_state.event_count {
                    let mut event_line = vec![Span::raw(format!("Events: {} ", count))];
                    if let Some(ref relative_time) = waiting_state.last_activity_relative {
                        event_line.push(Span::raw("| Last activity: "));
                        event_line.push(Span::styled(
                            relative_time.clone(),
                            Style::default().fg(Color::Green),
                        ));
                    }
                    lines.push(Line::from(event_line));
                }

                lines.push(Line::from(""));
                lines.push(Line::from("Tip: Use this command to export data:"));
                if let Some(ref session_id) = waiting_state.session_id {
                    lines.push(Line::from(Span::styled(
                        format!("  agtrace session show {} --json", session_id),
                        Style::default().fg(Color::Cyan),
                    )));
                } else {
                    lines.push(Line::from(Span::styled(
                        "  agtrace session show <id> --json",
                        Style::default().fg(Color::Cyan),
                    )));
                }

                lines
            }
            WaitingKind::MissingContext => {
                let mut lines = vec![Line::from("")];

                if let Some(ref session_id) = waiting_state.session_id {
                    lines.push(Line::from(vec![
                        Span::raw("Session: "),
                        Span::styled(
                            session_id.clone(),
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]));
                    lines.push(Line::from(""));
                }

                lines.push(Line::from(Span::styled(
                    "Unable to calculate saturation",
                    Style::default().fg(Color::Red),
                )));
                lines.push(Line::from(Span::styled(
                    "(waiting for model information...)",
                    Style::default().fg(Color::DarkGray),
                )));

                lines.push(Line::from(""));
                lines.push(Line::from("See raw events:"));
                lines.push(Line::from(Span::styled(
                    "  agtrace lab grep \".*\" --limit 10",
                    Style::default().fg(Color::Cyan),
                )));

                lines
            }
        };

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
