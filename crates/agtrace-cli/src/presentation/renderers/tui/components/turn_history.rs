use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use super::Component;
use crate::presentation::renderers::tui::app::AppState;

pub(crate) struct TurnHistoryComponent;

impl Component for TurnHistoryComponent {
    fn render(&self, f: &mut Frame, area: Rect, state: &mut AppState) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(Span::styled(
                " SATURATION HISTORY (Delta Highlight) ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ));

        if state.turns_usage.is_empty() || state.max_context.is_none() {
            let empty_list = List::new(vec![
                ListItem::new(Line::from("")),
                ListItem::new(Line::from(vec![Span::styled(
                    "Waiting for turn data...",
                    Style::default().fg(Color::DarkGray),
                )])),
            ])
            .block(block);
            f.render_widget(empty_list, area);
            return;
        }

        let max_context = state.max_context.unwrap() as f64;
        let inner_width = area.width.saturating_sub(4) as usize;

        let items: Vec<ListItem> = state
            .turns_usage
            .iter()
            .flat_map(|turn| render_turn(turn, max_context as u32, inner_width))
            .collect();

        let list = List::new(items).block(block);
        f.render_widget(list, area);
    }
}

fn render_turn(
    turn: &crate::presentation::view_models::TurnUsageViewModel,
    max_context: u32,
    inner_width: usize,
) -> Vec<ListItem<'static>> {
    let mut lines = Vec::new();

    if turn.is_active {
        lines.push(ListItem::new(Line::from(vec![Span::styled(
            "├─ CURRENT TURN (Active) ────────────────────────────────────",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )])));
    } else {
        lines.push(ListItem::new(Line::from("")));
    }

    let title_line = Line::from(vec![
        Span::styled(
            format!("#{:02} | ", turn.turn_id),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled("User: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(format!("\"{}\"", truncate_text(&turn.title, 50))),
    ]);
    lines.push(ListItem::new(title_line));

    if turn.is_active {
        if let Some(start_time) = turn.start_time {
            let now = chrono::Utc::now();
            let elapsed = now.signed_duration_since(start_time);
            let minutes = elapsed.num_minutes();
            let seconds = elapsed.num_seconds() % 60;

            let status_line = Line::from(vec![
                Span::styled("Status: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("In Progress ({}m {}s elapsed)", minutes, seconds),
                    Style::default().fg(Color::Yellow),
                ),
            ]);
            lines.push(ListItem::new(status_line));
        }
        lines.push(ListItem::new(Line::from("")));
    }

    let bar_line = create_stacked_bar(
        turn.prev_total,
        turn.delta,
        max_context,
        turn.is_heavy,
        inner_width,
    );
    lines.push(ListItem::new(bar_line));

    if turn.is_active && !turn.recent_steps.is_empty() {
        lines.push(ListItem::new(Line::from("")));
        lines.push(ListItem::new(Line::from(vec![Span::styled(
            "Recent Steps:",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )])));

        let steps: Vec<ListItem> = turn
            .recent_steps
            .iter()
            .map(|step| {
                let time_str = step.timestamp.format("%H:%M:%S").to_string();

                let mut spans = vec![
                    Span::styled(
                        format!("  {} ", time_str),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::raw(format!("{} ", step.emoji)),
                    Span::raw(step.description.clone()),
                ];

                if let Some(tokens) = step.token_usage {
                    spans.push(Span::styled(
                        format!(" (+{})", format_tokens(tokens as i32)),
                        Style::default().fg(Color::DarkGray),
                    ));
                }

                ListItem::new(Line::from(spans))
            })
            .collect();

        lines.extend(steps);
    }

    lines
}

fn create_stacked_bar(
    prev_total: u32,
    delta: u32,
    max_context: u32,
    is_heavy: bool,
    bar_width: usize,
) -> Line<'static> {
    let bar_width = bar_width.saturating_sub(20).min(60);

    let prev_ratio = prev_total as f64 / max_context as f64;
    let delta_ratio = delta as f64 / max_context as f64;

    let prev_chars = (prev_ratio * bar_width as f64) as usize;
    let delta_chars =
        (delta_ratio * bar_width as f64).max(if delta > 0 { 1.0 } else { 0.0 }) as usize;
    let remaining_chars = bar_width.saturating_sub(prev_chars + delta_chars);

    let history_bar = "█".repeat(prev_chars);
    let delta_bar = "▓".repeat(delta_chars);
    let void_bar = "░".repeat(remaining_chars);

    let delta_color = if is_heavy {
        Color::Red
    } else {
        Color::LightCyan
    };

    let pct = (delta as f64 / max_context as f64) * 100.0;
    let pct_text = if is_heavy {
        format!(" +{:.1}% (HEAVY!)", pct)
    } else {
        format!(" +{:.1}%", pct)
    };

    let mut spans = vec![Span::raw("      [")];

    if !history_bar.is_empty() {
        spans.push(Span::styled(
            history_bar,
            Style::default().fg(Color::DarkGray),
        ));
    }

    if !delta_bar.is_empty() {
        spans.push(Span::styled(
            delta_bar,
            Style::default()
                .fg(delta_color)
                .add_modifier(Modifier::BOLD),
        ));
    }

    if !void_bar.is_empty() {
        spans.push(Span::styled(
            void_bar,
            Style::default().fg(Color::Rgb(60, 60, 60)),
        ));
    }

    spans.push(Span::raw("]"));
    spans.push(Span::styled(
        pct_text,
        Style::default().fg(if is_heavy { Color::Red } else { Color::White }),
    ));

    Line::from(spans)
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

fn truncate_text(text: &str, max_len: usize) -> String {
    if text.chars().count() <= max_len {
        text.to_string()
    } else {
        let truncated: String = text.chars().take(max_len.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
}
