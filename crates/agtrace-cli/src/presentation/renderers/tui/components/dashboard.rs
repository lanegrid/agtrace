use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::Component;
use crate::presentation::renderers::tui::app::{AppState, WatchMode};

pub(crate) struct DashboardComponent;

impl Component for DashboardComponent {
    fn render(&self, f: &mut Frame, area: Rect, state: &mut AppState) {
        let has_context = state.context_usage.is_some();
        let has_session = state.attached_session_id.is_some();

        let mut constraints = vec![Constraint::Length(1)]; // Title bar

        if has_session {
            constraints.push(Constraint::Length(3)); // Status box (with borders)
        }

        if has_context {
            constraints.push(Constraint::Length(6)); // Context box (with borders)
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);

        let mut chunk_idx = 0;
        render_title_bar(f, chunks[chunk_idx], state);
        chunk_idx += 1;

        if has_session {
            render_status_box(f, chunks[chunk_idx], state);
            chunk_idx += 1;
        }

        if has_context {
            render_context_box(f, chunks[chunk_idx], state);
        }
    }
}

fn render_title_bar(f: &mut Frame, area: Rect, state: &AppState) {
    let title = Line::from(vec![
        Span::styled(
            "━━ ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "AgTrace Watch",
            Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
        ),
        if !state.session_title.is_empty() {
            Span::styled(
                format!(" → {}", state.session_title),
                Style::default().fg(Color::White),
            )
        } else {
            Span::raw("")
        },
        Span::styled(
            " ━━",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let mode_style = match state.mode {
        WatchMode::AutoFollow => Style::default()
            .fg(Color::LightRed)
            .add_modifier(Modifier::BOLD),
        WatchMode::Fixed => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    };

    let mode_text = match state.mode {
        WatchMode::AutoFollow => "🔴 LIVE",
        WatchMode::Fixed => "⏸️  PAUSED",
    };

    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(area);

    f.render_widget(Paragraph::new(title), layout[0]);
    f.render_widget(
        Paragraph::new(mode_text)
            .style(mode_style)
            .alignment(Alignment::Right),
        layout[1],
    );
}

fn render_status_box(f: &mut Frame, area: Rect, state: &AppState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green))
        .title(Span::styled(
            " Session Info ",
            Style::default()
                .fg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        ));

    let line = if state.attached_session_id.is_some() {
        let mut spans = Vec::new();

        if let Some(provider) = &state.provider_name {
            spans.push(Span::styled("Provider: ", Style::default().fg(Color::Gray)));
            spans.push(Span::styled(
                provider.clone(),
                Style::default()
                    .fg(Color::LightCyan)
                    .add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::raw(" │ "));
        }

        if let Some(session_id) = &state.attached_session_id {
            let short_id = if session_id.len() > 8 {
                &session_id[..8]
            } else {
                session_id
            };
            spans.push(Span::styled("Session: ", Style::default().fg(Color::Gray)));
            spans.push(Span::styled(
                short_id.to_string(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::raw(" │ "));
        }

        if let Some(model) = &state.model {
            spans.push(Span::styled("Model: ", Style::default().fg(Color::Gray)));
            spans.push(Span::styled(
                model.clone(),
                Style::default()
                    .fg(Color::LightMagenta)
                    .add_modifier(Modifier::BOLD),
            ));
        }

        Line::from(spans)
    } else {
        Line::from(vec![Span::styled(
            "Waiting for session...",
            Style::default().fg(Color::DarkGray),
        )])
    };

    let paragraph = Paragraph::new(line).block(block);
    f.render_widget(paragraph, area);
}

fn render_context_box(f: &mut Frame, area: Rect, state: &AppState) {
    if let Some(ctx) = &state.context_usage {
        let ratio = ctx.used as f64 / ctx.limit as f64;
        let usage_pct = ratio * 100.0;

        let (gauge_color, border_color) = if ratio > 0.9 {
            (Color::Red, Color::LightRed)
        } else if ratio > 0.7 {
            (Color::Rgb(255, 165, 0), Color::Rgb(255, 200, 100)) // Orange instead of yellow
        } else {
            (Color::Green, Color::Cyan)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title(Span::styled(
                " Context Window ",
                Style::default()
                    .fg(border_color)
                    .add_modifier(Modifier::BOLD),
            ));

        let inner = block.inner(area);
        f.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Usage percentage (big)
                Constraint::Length(1), // Progress bar
                Constraint::Length(1), // Token counts
                Constraint::Length(1), // Compaction status
            ])
            .split(inner);

        // Usage percentage (large and prominent)
        let pct_line = Line::from(vec![Span::styled(
            format!("{:.0}% used", usage_pct),
            Style::default()
                .fg(gauge_color)
                .add_modifier(Modifier::BOLD),
        )]);
        f.render_widget(Paragraph::new(pct_line), chunks[0]);

        // Compact progress bar (40 chars max)
        let bar_width = 40;
        let filled = ((ratio * bar_width as f64) as usize).min(bar_width);
        let empty = bar_width.saturating_sub(filled);
        let bar = format!("[{}{}]", "█".repeat(filled), "░".repeat(empty));

        let bar_line = Line::from(vec![Span::styled(bar, Style::default().fg(gauge_color))]);
        f.render_widget(Paragraph::new(bar_line), chunks[1]);

        // Token counts (simplified)
        let input_total = ctx.fresh_input + ctx.cache_creation + ctx.cache_read;
        let counts_line = Line::from(vec![
            Span::styled(
                format!(
                    "{} / {} tokens",
                    format_tokens(ctx.used as i32),
                    format_tokens(ctx.limit as i32)
                ),
                Style::default().fg(Color::White),
            ),
            Span::styled("  •  ", Style::default().fg(Color::DarkGray)),
            Span::styled("In: ", Style::default().fg(Color::Gray)),
            Span::styled(format_tokens(input_total), Style::default().fg(Color::Cyan)),
            Span::styled("  Out: ", Style::default().fg(Color::Gray)),
            Span::styled(format_tokens(ctx.output), Style::default().fg(Color::Green)),
        ]);
        f.render_widget(Paragraph::new(counts_line), chunks[2]);

        // Compaction status (simplified)
        if let Some(buffer_pct) = state.compaction_buffer_pct {
            if buffer_pct > 0.0 {
                let trigger_threshold = 100.0 - buffer_pct;
                let current_input_pct = (input_total as f64 / ctx.limit as f64) * 100.0;

                let (icon, text, color) = if current_input_pct >= trigger_threshold {
                    ("⚠️", "Near limit", Color::Rgb(255, 165, 0)) // Orange
                } else {
                    ("✓", "OK", Color::Green)
                };

                let status_line = Line::from(vec![
                    Span::raw(format!("{} ", icon)),
                    Span::styled(
                        text,
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    ),
                ]);

                f.render_widget(Paragraph::new(status_line), chunks[3]);
            }
        }
    }
}

fn format_tokens(count: i32) -> String {
    if count >= 1_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    } else if count >= 1_000 {
        format!("{:.1}k", count as f64 / 1_000.0)
    } else {
        count.to_string()
    }
}
