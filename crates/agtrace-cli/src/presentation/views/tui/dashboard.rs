//! Dashboard View Component
//!
//! Renders the top section with session overview and context usage.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Widget},
};

use crate::presentation::view_models::DashboardViewModel;

use super::status_level_to_color;

/// Dashboard view wrapper
pub struct DashboardView<'a> {
    model: &'a DashboardViewModel,
}

impl<'a> DashboardView<'a> {
    pub fn new(model: &'a DashboardViewModel) -> Self {
        Self { model }
    }
}

impl<'a> Widget for DashboardView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        use ratatui::style::Color;

        let title = if let Some(sub_title) = &self.model.sub_title {
            format!("{} - {}", self.model.title, sub_title)
        } else {
            self.model.title.clone()
        };

        let title_style = if self.model.sub_title.is_some() {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let block = Block::default()
            .title(title)
            .title_style(title_style)
            .borders(Borders::ALL)
            .style(Style::default());

        let inner = block.inner(area);
        block.render(area, buf);

        // Split into info section and context gauge
        let chunks = Layout::vertical([Constraint::Length(4), Constraint::Length(3)]).split(inner);

        // Info section
        self.render_info_section(chunks[0], buf);

        // Context gauge
        self.render_context_gauge(chunks[1], buf);
    }
}

impl<'a> DashboardView<'a> {
    fn render_info_section(&self, area: Rect, buf: &mut Buffer) {
        let mut lines = vec![];

        // Session ID and model
        let session_line = Line::from(vec![
            Span::styled("Session: ", Style::default().add_modifier(Modifier::DIM)),
            Span::raw(&self.model.session_id),
        ]);
        lines.push(session_line);

        // Model
        if let Some(model) = &self.model.model {
            let model_line = Line::from(vec![
                Span::styled("Model: ", Style::default().add_modifier(Modifier::DIM)),
                Span::raw(model),
            ]);
            lines.push(model_line);
        }

        // Project root
        if let Some(project) = &self.model.project_root {
            let project_line = Line::from(vec![
                Span::styled("Project: ", Style::default().add_modifier(Modifier::DIM)),
                Span::raw(project),
            ]);
            lines.push(project_line);
        }

        // Elapsed time
        let elapsed_str = format_duration(self.model.elapsed_seconds);
        let elapsed_line = Line::from(vec![
            Span::styled("Elapsed: ", Style::default().add_modifier(Modifier::DIM)),
            Span::raw(elapsed_str),
        ]);
        lines.push(elapsed_line);

        let paragraph = Paragraph::new(lines);
        paragraph.render(area, buf);
    }

    fn render_context_gauge(&self, area: Rect, buf: &mut Buffer) {
        use ratatui::style::Color;

        let color = status_level_to_color(self.model.context_color);
        let total_formatted = format_tokens(self.model.context_total);

        // Handle missing context limit explicitly
        let gauge = if let (Some(limit), Some(usage_pct)) =
            (self.model.context_limit, self.model.context_usage_pct)
        {
            let limit_formatted = format_tokens(limit);
            let remaining = limit.saturating_sub(self.model.context_total);
            let remaining_formatted = format_tokens(remaining);

            Gauge::default()
                .block(Block::default().style(Style::default().bg(Color::Rgb(40, 40, 40))))
                .gauge_style(Style::default().fg(color).add_modifier(Modifier::BOLD))
                .ratio(usage_pct)
                .label(format!(
                    "LIFE: {} / {} ({:.0}%) - {} remaining",
                    total_formatted,
                    limit_formatted,
                    usage_pct * 100.0,
                    remaining_formatted
                ))
        } else {
            // No limit known - display warning
            Gauge::default()
                .block(Block::default().style(Style::default().bg(Color::Rgb(40, 40, 40))))
                .gauge_style(Style::default().fg(color).add_modifier(Modifier::BOLD))
                .ratio(0.0)
                .label(format!(
                    "LIFE: {} / ??? (limit unknown - check model config)",
                    total_formatted
                ))
        };

        gauge.render(area, buf);
    }
}

/// Format duration in seconds to human-readable string
fn format_duration(seconds: u64) -> String {
    if seconds < 60 {
        format!("{}s", seconds)
    } else if seconds < 3600 {
        format!("{}m {}s", seconds / 60, seconds % 60)
    } else {
        format!("{}h {}m", seconds / 3600, (seconds % 3600) / 60)
    }
}

/// Format token count with k/M suffixes
fn format_tokens(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}k", tokens as f64 / 1_000.0)
    } else {
        tokens.to_string()
    }
}
